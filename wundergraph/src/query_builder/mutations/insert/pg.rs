use super::{HandleBatchInsert, HandleInsert};
use crate::context::WundergraphContext;
use crate::helper::primary_keys::UnRef;
use crate::query_builder::selection::fields::WundergraphFieldList;
use crate::query_builder::selection::filter::build_filter::BuildFilter;
use crate::query_builder::selection::order::BuildOrder;
use crate::query_builder::selection::query_modifier::QueryModifier;
use crate::query_builder::selection::select::BuildSelect;
use crate::query_builder::selection::{LoadingHandler, SqlTypeOfPlaceholder};
use crate::scalar::WundergraphScalarValue;
use diesel::associations::HasTable;
use diesel::dsl::SqlTypeOf;
use diesel::expression::{Expression, NonAggregate, SelectableExpression};
use diesel::insertable::CanInsertInSingleQuery;
use diesel::pg::Pg;
use diesel::query_builder::{BoxedSelectStatement, QueryFragment};
use diesel::query_dsl::methods::{BoxedDsl, FilterDsl, OrFilterDsl};
use diesel::sql_types::HasSqlType;
use diesel::{AppearsOnTable, Connection, Insertable, RunQueryDsl, Table};
use diesel::{EqAll, Identifiable, Queryable};
use juniper::{ExecutionResult, Executor, Selection, Value};

impl<I, Ctx, L, T, Id> HandleInsert<L, I, Pg, Ctx> for T
where
    T: Table + HasTable<Table = T> + 'static,
    T::FromClause: QueryFragment<Pg>,
    L: LoadingHandler<Pg, Ctx, Table = T> + 'static,
    L::Columns: BuildOrder<T, Pg>
        + BuildSelect<
            T,
            Pg,
            SqlTypeOfPlaceholder<L::FieldList, Pg, L::PrimaryKeyIndex, T, Ctx>,
        >,
    Ctx: WundergraphContext + QueryModifier<L, Pg>,
    Ctx::Connection: Connection<Backend = Pg>,
    L::FieldList: WundergraphFieldList<Pg, L::PrimaryKeyIndex, T, Ctx>,
    I: Insertable<T>,
    I::Values: QueryFragment<Pg> + CanInsertInSingleQuery<Pg>,
    T::PrimaryKey: QueryFragment<Pg>,
    T: BoxedDsl<
        'static,
        Pg,
        Output = BoxedSelectStatement<'static, SqlTypeOf<<T as Table>::AllColumns>, T, Pg>,
    >,
    <Ctx::Connection as Connection>::Backend:
        HasSqlType<SqlTypeOf<T::PrimaryKey>>
            + HasSqlType<SqlTypeOfPlaceholder<L::FieldList, Pg, L::PrimaryKeyIndex, T, Ctx>>,
    <L::Filter as BuildFilter<Pg>>::Ret: AppearsOnTable<T>,
    T::PrimaryKey: EqAll<Id>,
    &'static L: Identifiable,
    <&'static L as Identifiable>::Id: UnRef<'static, UnRefed = Id>,
    Id: Queryable<<T::PrimaryKey as Expression>::SqlType, Pg>,
    <T::PrimaryKey as EqAll<Id>>::Output:
        SelectableExpression<T> + NonAggregate + QueryFragment<Pg> + 'static,
{
    fn handle_insert(
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        insertable: I,
    ) -> ExecutionResult<WundergraphScalarValue> {
        let ctx = executor.context();
        let conn = ctx.get_connection();
        conn.transaction(|| -> ExecutionResult<WundergraphScalarValue> {
            let look_ahead = executor.look_ahead();
            let inserted = insertable
                .insert_into(Self::table())
                .returning(Self::table().primary_key());
            if cfg!(feature = "debug") {
                println!("{}", ::diesel::debug_query(&inserted));
            }
            let inserted: Id = inserted.get_result(conn)?;
            let q = L::build_query(&look_ahead)?;
            let q = FilterDsl::filter(q, Self::table().primary_key().eq_all(inserted));
            let items = L::load(&look_ahead, selection, executor, q)?;
            Ok(items.into_iter().next().unwrap_or(Value::Null))
        })
    }
}

impl<I, Ctx, L, T, Id> HandleBatchInsert<L, I, Pg, Ctx> for T
where
    T: Table + HasTable<Table = T> + 'static,
    T::FromClause: QueryFragment<Pg>,
    L: LoadingHandler<Pg, Ctx, Table = T> + 'static,
    L::Columns: BuildOrder<T, Pg>
        + BuildSelect<
            T,
            Pg,
            SqlTypeOfPlaceholder<L::FieldList, Pg, L::PrimaryKeyIndex, T, Ctx>,
        >,
    Ctx: WundergraphContext + QueryModifier<L, Pg>,
    Ctx::Connection: Connection<Backend = Pg>,
    L::FieldList: WundergraphFieldList<Pg, L::PrimaryKeyIndex, T, Ctx>,
    Vec<I>: Insertable<T>,
    <Vec<I> as Insertable<T>>::Values: QueryFragment<Pg> + CanInsertInSingleQuery<Pg>,
    T::PrimaryKey: QueryFragment<Pg>,
    T: BoxedDsl<
        'static,
        Pg,
        Output = BoxedSelectStatement<'static, SqlTypeOf<<T as Table>::AllColumns>, T, Pg>,
    >,
    <Ctx::Connection as Connection>::Backend:
        HasSqlType<SqlTypeOf<T::PrimaryKey>>
            + HasSqlType<SqlTypeOfPlaceholder<L::FieldList, Pg, L::PrimaryKeyIndex, T, Ctx>>,
    <L::Filter as BuildFilter<Pg>>::Ret: AppearsOnTable<T>,
    T::PrimaryKey: EqAll<Id>,
    &'static L: Identifiable,
    <&'static L as Identifiable>::Id: UnRef<'static, UnRefed = Id>,
    Id: Queryable<<T::PrimaryKey as Expression>::SqlType, Pg>,
    <T::PrimaryKey as EqAll<Id>>::Output:
        SelectableExpression<T> + NonAggregate + QueryFragment<Pg> + 'static,
{
    fn handle_batch_insert(
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        batch: Vec<I>,
    ) -> ExecutionResult<WundergraphScalarValue> {
        let ctx = executor.context();
        let conn = ctx.get_connection();
        conn.transaction(|| -> ExecutionResult<WundergraphScalarValue> {
            let look_ahead = executor.look_ahead();
            let inserted = batch
                .insert_into(Self::table())
                .returning(Self::table().primary_key());
            if cfg!(feature = "debug") {
                println!("{}", ::diesel::debug_query(&inserted));
            }
            let inserted: Vec<Id> = inserted.get_results(conn)?;
            let mut q = L::build_query(&look_ahead)?;
            for i in inserted {
                q = OrFilterDsl::or_filter(q, Self::table().primary_key().eq_all(i));
            }
            let items = L::load(&look_ahead, selection, executor, q)?;
            Ok(Value::list(items))
        })
    }
}
