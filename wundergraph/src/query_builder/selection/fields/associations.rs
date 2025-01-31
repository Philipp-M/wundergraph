use super::WundergraphFieldList;
use crate::query_builder::selection::offset::ApplyOffset;
use crate::query_builder::selection::LoadingHandler;
use crate::query_builder::types::HasMany;
use crate::query_builder::selection::query_resolver::get_sub_field;
use crate::scalar::WundergraphScalarValue;
use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::expression::NonAggregate;
use diesel::query_builder::QueryFragment;
use diesel::{QuerySource, SelectableExpression};
use failure::Error;
use juniper::{Executor, LookAheadMethods, Selection};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub struct AssociationsReturn<'a, K: Eq + Hash> {
    keys: Vec<Option<K>>,
    fields: Vec<&'a str>,
    values: HashMap<Option<K>, Vec<(usize, Vec<juniper::Value<WundergraphScalarValue>>)>>,
}

impl<'a, K: Eq + Hash> AssociationsReturn<'a, K> {
    fn empty() -> Self {
        Self {
            keys: Vec::new(),
            fields: Vec::new(),
            values: HashMap::new(),
        }
    }

    fn init(&mut self, get_keys: &impl Fn() -> Vec<Option<K>>) {
        if self.keys.is_empty() {
            self.keys = get_keys()
        }
    }

    fn push_field<T, O, DB, Ctx>(
        &mut self,
        field: &'static str,
        look_ahead: &juniper::LookAheadSelection<'a, WundergraphScalarValue>,
        selection: Option<&'a [Selection<'a, WundergraphScalarValue>]>,
        executor: &'a Executor<'a, Ctx, WundergraphScalarValue>,
    ) -> Result<(), Error>
    where
        DB: Backend,
        T: WundergraphResolveAssociation<K, O, DB, Ctx>,
    {
        let (name, alias, loc, selection) = get_sub_field(field, selection);
        let executor = executor.field_sub_executor(alias, name, loc, selection);

        let values = T::resolve(look_ahead, selection, &self.keys, &executor)?;

        let len = self.fields.len();
        self.fields.push(alias);

        for (k, v) in values {
            self.values.entry(k).or_insert_with(Vec::new).push((len, v));
        }
        Ok(())
    }

    pub(crate) fn merge_with_object_list(
        self,
        objs: Vec<juniper::Object<WundergraphScalarValue>>,
    ) -> Vec<juniper::Value<WundergraphScalarValue>> {
        let Self {
            values,
            keys,
            fields,
        } = self;
        if keys.is_empty() {
            objs.into_iter().map(juniper::Value::object).collect()
        } else {
            objs.into_iter()
                .zip(keys.into_iter())
                .map(|(mut obj, key)| {
                    let values = values.get(&key);
                    if let Some(values) = values {
                        let mut value_iter = values.iter().peekable();
                        for (idx, field_name) in fields.iter().enumerate() {
                            match value_iter.peek() {
                                Some((field_idx, _)) if idx == *field_idx => {
                                    let value = value_iter
                                        .next()
                                        .expect("It's there because peekable")
                                        .1
                                        .clone();
                                    obj.add_field(
                                        field_name.to_owned(),
                                        juniper::Value::List(value),
                                    );
                                }
                                None | Some(_) => {
                                    obj.add_field(
                                        field_name.to_owned(),
                                        juniper::Value::List(Vec::new()),
                                    );
                                }
                            }
                        }
                    } else {
                        for f in &fields {
                            obj.add_field(f.to_owned(), juniper::Value::List(Vec::new()));
                        }
                    }
                    obj
                })
                .map(juniper::Value::object)
                .collect()
        }
    }
}

pub trait WundergraphResolveAssociations<K, Other, DB, Ctx>
where
    K: Eq + Hash,
    DB: Backend,
{
    fn resolve<'a>(
        look_ahead: &'a juniper::LookAheadSelection<'a, WundergraphScalarValue>,
        selection: Option<&'a [Selection<'a, WundergraphScalarValue>]>,
        get_name: impl Fn(usize) -> &'static str,
        get_keys: impl Fn() -> Vec<Option<K>>,
        executor: &'a Executor<'a, Ctx, WundergraphScalarValue>,
    ) -> Result<AssociationsReturn<'a, K>, Error>;
}

impl<K, Other, DB, Ctx> WundergraphResolveAssociations<K, Other, DB, Ctx> for ()
where
    K: Eq + Hash,
    DB: Backend,
{
    fn resolve<'a>(
        _look_ahead: &'a juniper::LookAheadSelection<'a, WundergraphScalarValue>,
        _selection: Option<&'a [Selection<'a, WundergraphScalarValue>]>,
        _get_name: impl Fn(usize) -> &'static str,
        _get_keys: impl Fn() -> Vec<Option<K>>,
        _executor: &'a Executor<'a, Ctx, WundergraphScalarValue>,
    ) -> Result<AssociationsReturn<'a, K>, Error> {
        Ok(AssociationsReturn::empty())
    }
}

pub trait WundergraphResolveAssociation<K, Other, DB: Backend, Ctx> {
    fn resolve(
        look_ahead: &juniper::LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        primary_keys: &[Option<K>],
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    ) -> Result<HashMap<Option<K>, Vec<juniper::Value<WundergraphScalarValue>>>, Error>;
}

pub trait WundergraphBelongsTo<Other, DB, Ctx, FK>: LoadingHandler<DB, Ctx>
where
    DB: Backend + ApplyOffset + 'static,
    Self::Table: 'static,
    <Self::Table as QuerySource>::FromClause: QueryFragment<DB>,
    DB::QueryBuilder: Default,
    FK: Default + NonAggregate + SelectableExpression<Self::Table> + QueryFragment<DB>,
{
    type Key: Eq + Hash;

    fn resolve(
        selection: &juniper::LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        keys: &[Option<Self::Key>],
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    ) -> Result<HashMap<Option<Self::Key>, Vec<juniper::Value<WundergraphScalarValue>>>, Error>;

    fn build_response(
        res: Vec<(
            Option<Self::Key>,
            <Self::FieldList as WundergraphFieldList<
                DB,
                Self::PrimaryKeyIndex,
                Self::Table,
                Ctx,
            >>::PlaceHolder,
        )>,
        look_ahead: &juniper::LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    ) -> Result<HashMap<Option<Self::Key>, Vec<juniper::Value<WundergraphScalarValue>>>, Error>
    {
        let (keys, vals): (Vec<_>, Vec<_>) = res.into_iter().unzip();
        let vals = <<Self as LoadingHandler<DB, Ctx>>::FieldList as WundergraphFieldList<
            DB,
            <Self as LoadingHandler<DB, Ctx>>::PrimaryKeyIndex,
            <Self as HasTable>::Table,
            Ctx,
        >>::resolve(
            vals,
            look_ahead,
            selection,
            <Self as LoadingHandler<DB, Ctx>>::FIELD_NAMES,
            executor,
        )?;
        Ok(keys
            .into_iter()
            .zip(vals.into_iter())
            .fold(HashMap::new(), |mut m, (k, v)| {
                (*m.entry(k).or_insert_with(Vec::new)).push(v);
                m
            }))
    }
}

impl<T, K, Other, DB, Ctx, FK> WundergraphResolveAssociation<K, Other, DB, Ctx> for HasMany<T, FK>
where
    DB: Backend + ApplyOffset + 'static,
    FK: Default + NonAggregate + QueryFragment<DB> + SelectableExpression<T::Table>,
    T: WundergraphBelongsTo<Other, DB, Ctx, FK, Key = K>,
    K: Eq + Hash,
    T::Table: 'static,
    <T::Table as QuerySource>::FromClause: QueryFragment<DB>,
    DB::QueryBuilder: Default,
{
    fn resolve(
        look_ahead: &juniper::LookAheadSelection<'_, WundergraphScalarValue>,
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        primary_keys: &[Option<K>],
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    ) -> Result<HashMap<Option<K>, Vec<juniper::Value<WundergraphScalarValue>>>, Error> {
        T::resolve(look_ahead, selection, primary_keys, executor)
    }
}

macro_rules! wundergraph_impl_resolve_association {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST: ident, $TT: ident,) +
        }
    )+) => {
        $(
            impl<Key, Back, Other, Ctx, $($T,)*> WundergraphResolveAssociations<Key, Other, Back, Ctx> for ($($T,)*)
            where Back: Backend,
                  Key: Eq + Hash,
                $($T: WundergraphResolveAssociation<Key, Other, Back, Ctx>,)*

            {
                fn resolve<'a>(
                    look_ahead: &'a juniper::LookAheadSelection<'a, WundergraphScalarValue>,
                    selection: Option<&'a [Selection<'a, WundergraphScalarValue>]>,
                    get_name: impl Fn(usize) -> &'static str,
                    get_keys: impl Fn() -> Vec<Option<Key>>,
                    executor: &'a Executor<'a, Ctx, WundergraphScalarValue>,
                ) -> Result<AssociationsReturn<'a, Key>, Error>
                {
                    let mut ret = AssociationsReturn::empty();
                    $(
                        if let Some(look_ahead) = look_ahead.select_child(get_name($idx)) {
                            ret.init(&get_keys);
                            ret.push_field::<$T, Other, Back, Ctx>(get_name($idx), look_ahead, selection, executor)?;
                        }
                    )*
                    Ok(ret)
                }
            }
        )*
    }
}

__diesel_for_each_tuple!(wundergraph_impl_resolve_association);
