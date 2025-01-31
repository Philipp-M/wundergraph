use crate::context::WundergraphContext;
use crate::diesel_ext::BoxableFilter;
use crate::error::WundergraphError;
use crate::helper::primary_keys::{PrimaryKeyArgument, UnRef};
use crate::juniper_ext::FromLookAheadValue;
use crate::query_builder::selection::order::BuildOrder;
use crate::query_builder::selection::query_modifier::QueryModifier;
use crate::query_builder::selection::select::BuildSelect;
use crate::helper::tuple::IsPrimaryKeyIndex;
use crate::scalar::WundergraphScalarValue;
use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::dsl::SqlTypeOf;
use diesel::expression::NonAggregate;
use diesel::query_builder::{BoxedSelectStatement, QueryFragment};
use diesel::query_dsl::methods::BoxedDsl;
use diesel::query_dsl::methods::FilterDsl;
use diesel::query_dsl::methods::{LimitDsl, SelectDsl};
use diesel::sql_types::{Bool, HasSqlType};
use diesel::BoxableExpression;
use diesel::EqAll;
use diesel::Identifiable;
use diesel::QuerySource;
use diesel::{AppearsOnTable, Connection, QueryDsl, Table};
use failure::Error;
use juniper::LookAheadValue;
use juniper::{Executor, LookAheadSelection, Selection};

pub mod fields;
pub mod filter;
pub mod offset;
pub mod order;
pub mod query_modifier;
pub mod select;
pub mod query_resolver;

use self::fields::WundergraphFieldList;
use self::filter::build_filter::BuildFilter;
use self::filter::inner_filter::InnerFilter;
use self::filter::Filter;
use self::offset::ApplyOffset;

#[doc(inline)]
pub use self::query_resolver::SqlTypeOfPlaceholder;

pub type BoxedQuery<'a, L, DB, Ctx> = BoxedSelectStatement<
    'a,
    SqlTypeOfPlaceholder<
        <L as LoadingHandler<DB, Ctx>>::FieldList,
        DB,
        <L as LoadingHandler<DB, Ctx>>::PrimaryKeyIndex,
        <L as HasTable>::Table,
        Ctx,
    >,
    <L as HasTable>::Table,
    DB,
>;

pub trait LoadingHandler<DB, Ctx>: HasTable + Sized
where
    DB: Backend + ApplyOffset + 'static,
{
    type Columns: BuildOrder<Self::Table, DB>
        + BuildSelect<
            Self::Table,
            DB,
            SqlTypeOfPlaceholder<Self::FieldList, DB, Self::PrimaryKeyIndex, Self::Table, Ctx>,
        >;
    type FieldList: WundergraphFieldList<DB, Self::PrimaryKeyIndex, Self::Table, Ctx>;

    type PrimaryKeyIndex: Default + IsPrimaryKeyIndex;
    type Filter: InnerFilter + BuildFilter<DB> + 'static;

    const FIELD_NAMES: &'static [&'static str];
    const TYPE_NAME: &'static str;

    fn load<'a>(
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        query: BoxedQuery<'a, Self, DB, Ctx>,
    ) -> Result<Vec<juniper::Value<WundergraphScalarValue>>, Error>
    where
        DB: HasSqlType<
            SqlTypeOfPlaceholder<Self::FieldList, DB, Self::PrimaryKeyIndex, Self::Table, Ctx>,
        >,
        Ctx: WundergraphContext + QueryModifier<Self, DB>,
        Ctx::Connection: Connection<Backend = DB>,
        DB::QueryBuilder: Default,
        <Self::Table as QuerySource>::FromClause: QueryFragment<DB>,
    {
        use diesel::RunQueryDsl;
        let ctx = executor.context();
        let conn = ctx.get_connection();
        let query = ctx.modify_query(select, query)?;
        if cfg!(feature = "debug") {
            #[allow(clippy::use_debug, clippy::print_stdout)]
            {
                println!("{:?}", diesel::debug_query(&query));
            }
        }
        let placeholder = <_ as RunQueryDsl<_>>::load(query, conn)?;
        Ok(Self::FieldList::resolve(
            placeholder,
            select,
            selection,
            Self::FIELD_NAMES,
            executor,
        )?)
    }

    fn load_by_primary_key<'a>(
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        mut query: BoxedQuery<'a, Self, DB, Ctx>,
    ) -> Result<Option<juniper::Value<WundergraphScalarValue>>, Error>
    where
        Self: 'static,
        &'static Self: Identifiable,
        Ctx: WundergraphContext + QueryModifier<Self, DB>,
        Ctx::Connection: Connection<Backend = DB>,
        <&'static Self as Identifiable>::Id: UnRef<'static>,
        <Self::Table as Table>::PrimaryKey:
            EqAll<<<&'static Self as Identifiable>::Id as UnRef<'static>>::UnRefed>,
        <<Self::Table as Table>::PrimaryKey as EqAll<
            <<&'static Self as Identifiable>::Id as UnRef<'static>>::UnRefed,
        >>::Output: AppearsOnTable<Self::Table> + NonAggregate + QueryFragment<DB>,
        PrimaryKeyArgument<'static, Self::Table, (), <&'static Self as Identifiable>::Id>:
            FromLookAheadValue,
        DB: HasSqlType<
            SqlTypeOfPlaceholder<Self::FieldList, DB, Self::PrimaryKeyIndex, Self::Table, Ctx>,
        >,
        DB::QueryBuilder: Default,
        <Self::Table as QuerySource>::FromClause: QueryFragment<DB>,
    {
        use juniper::LookAheadMethods;
        let v = select
            .argument("primaryKey")
            .ok_or(WundergraphError::NoPrimaryKeyArgumentFound)?;
        let key = PrimaryKeyArgument::<
            Self::Table,
            _,
            <&'static Self as Identifiable>::Id,
            >::from_look_ahead(v.value())
            .ok_or(WundergraphError::NoPrimaryKeyArgumentFound)?;
        query = <_ as QueryDsl>::filter(query, Self::table().primary_key().eq_all(key.values));
        query = <_ as QueryDsl>::limit(query, 1);
        let res = Self::load(select, selection, executor, query)?;
        Ok(res.into_iter().next())
    }

    fn build_query<'a>(
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<BoxedQuery<'a, Self, DB, Ctx>, Error>
    where
        Self::Table: BoxedDsl<
                'a,
                DB,
                Output = BoxedSelectStatement<
                    'a,
                    SqlTypeOf<<Self::Table as Table>::AllColumns>,
                    Self::Table,
                    DB,
                >,
            > + 'static,
        <Self::Filter as BuildFilter<DB>>::Ret: AppearsOnTable<Self::Table>,
    {
        let mut query =
            <_ as SelectDsl<_>>::select(Self::table().into_boxed(), Self::get_select(select)?);

        query = Self::apply_filter(query, select)?;
        query = Self::apply_limit(query, select)?;
        query = Self::apply_offset(query, select)?;
        query = Self::apply_order(query, select)?;

        Ok(query)
    }

    fn get_select(
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<
        Box<
            dyn BoxableExpression<
                Self::Table,
                DB,
                SqlType = SqlTypeOfPlaceholder<
                    Self::FieldList,
                    DB,
                    Self::PrimaryKeyIndex,
                    Self::Table,
                    Ctx,
                >,
            >,
        >,
        Error,
    > {
        use juniper::LookAheadMethods;
        <Self::Columns as BuildSelect<Self::Table, DB, _>>::build_select(
            select,
            |local_index| {
                Self::FieldList::map_table_field(local_index, |global| Self::FIELD_NAMES[global])
                    .expect("Field is there")
            },
            Self::PrimaryKeyIndex::is_index,
            (0..Self::FieldList::NON_TABLE_FIELD_COUNT).any(|i| {
                Self::FieldList::map_non_table_field(i, |global| {
                    select.has_child(Self::FIELD_NAMES[global])
                })
                .unwrap_or(false)
            }),
        )
    }

    fn get_filter(
        input: &LookAheadValue<'_, WundergraphScalarValue>,
    ) -> Result<Option<Box<dyn BoxableFilter<Self::Table, DB, SqlType = Bool>>>, Error>
    where
        Self::Table: 'static,
        <Self::Filter as BuildFilter<DB>>::Ret: AppearsOnTable<Self::Table>,
    {
        Ok(
            <Filter<Self::Filter, Self::Table> as FromLookAheadValue>::from_look_ahead(input)
                .and_then(<_ as BuildFilter<DB>>::into_filter),
        )
    }

    fn apply_filter<'a>(
        query: BoxedQuery<'a, Self, DB, Ctx>,
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<BoxedQuery<'a, Self, DB, Ctx>, Error>
    where
        Self::Table: 'static,
        <Self::Filter as BuildFilter<DB>>::Ret: AppearsOnTable<Self::Table>,
    {
        use juniper::LookAheadMethods;
        if let Some(filter) = select.argument("filter") {
            if let Some(filter) = Self::get_filter(filter.value())? {
                Ok(<_ as FilterDsl<_>>::filter(query, filter))
            } else {
                Ok(query)
            }
        } else {
            Ok(query)
        }
    }

    fn apply_order<'a>(
        mut query: BoxedQuery<'a, Self, DB, Ctx>,
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<BoxedQuery<'a, Self, DB, Ctx>, Error>
    where
        Self::Table: 'static,
    {
        use juniper::{LookAheadArgument, LookAheadMethods};
        if let Some(LookAheadValue::List(order)) =
            select.argument("order").map(LookAheadArgument::value)
        {
            let order_stmts = <Self::Columns as BuildOrder<Self::Table, DB>>::build_order(
                order,
                |local_index| {
                    Self::FieldList::map_table_field(local_index, |global| {
                        Self::FIELD_NAMES[global]
                    })
                    .expect("Field is there")
                },
            )?;
            for s in order_stmts {
                query = query.then_order_by(s);
            }
            Ok(query)
        } else {
            Ok(query)
        }
    }

    fn apply_limit<'a>(
        query: BoxedQuery<'a, Self, DB, Ctx>,
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<BoxedQuery<'a, Self, DB, Ctx>, Error> {
        use juniper::LookAheadMethods;
        if let Some(limit) = select.argument("limit") {
            Ok(<_ as LimitDsl>::limit(
                query,
                i64::from_look_ahead(limit.value())
                    .ok_or(WundergraphError::CouldNotBuildFilterArgument)?,
            ))
        } else {
            Ok(query)
        }
    }

    fn apply_offset<'a>(
        query: BoxedQuery<'a, Self, DB, Ctx>,
        select: &LookAheadSelection<'_, WundergraphScalarValue>,
    ) -> Result<BoxedQuery<'a, Self, DB, Ctx>, Error> {
        <DB as ApplyOffset>::apply_offset::<Self, Ctx>(query, select)
    }

    fn field_description(_idx: usize) -> Option<&'static str> {
        None
    }

    fn type_description() -> Option<&'static str> {
        None
    }

    #[allow(clippy::option_option)]
    fn field_deprecation(_idx: usize) -> Option<Option<&'static str>> {
        None
    }
}
