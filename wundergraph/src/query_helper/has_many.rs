use diesel::associations::BelongsTo;
use diesel::backend::Backend;
use diesel::sql_types::{Bool, Nullable};
use diesel::Queryable;
use juniper::meta::MetaType;
use juniper::{
    Arguments, ExecutionResult, Executor, FieldError, GraphQLType, Registry, Selection, Value,
};
use scalar::WundergraphScalarValue;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum HasMany<T> {
    NotLoaded,
    Items(Vec<T>),
}

impl<T, P> BelongsTo<P> for HasMany<T> where T: BelongsTo<P> {
    type ForeignKey = T::ForeignKey;
    type ForeignKeyColumn = T::ForeignKeyColumn;

    fn foreign_key(&self) -> Option<&Self::ForeignKey> {
        unimplemented!()
    }

    fn foreign_key_column() -> Self::ForeignKeyColumn {
        T::foreign_key_column()
    }
}

impl<T> Default for HasMany<T> {
    fn default() -> Self {
        HasMany::NotLoaded
    }
}

impl<T> HasMany<T> {
    pub fn expect_items(&self, msg: &str) -> &[T] {
        if let HasMany::Items(ref i) = *self {
            i
        } else {
            panic!("{}", msg)
        }
    }
}

impl<DB, T> Queryable<Nullable<Bool>, DB> for HasMany<T>
where
    DB: Backend,
    bool: Queryable<Bool, DB>,
{
    type Row = <Option<bool> as Queryable<Nullable<Bool>, DB>>::Row;

    fn build(row: Self::Row) -> Self {
        assert!(<Option<bool> as Queryable<_, _>>::build(row).is_none());
        HasMany::NotLoaded
    }
}

impl<T> GraphQLType<WundergraphScalarValue> for HasMany<T>
where
    T: GraphQLType<WundergraphScalarValue>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Vec::<T>::name(info)
    }

    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut Registry<'r, WundergraphScalarValue>,
    ) -> MetaType<'r, WundergraphScalarValue>
    where
        WundergraphScalarValue: 'r,
    {
        Vec::<T>::meta(info, registry)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        arguments: &Arguments<WundergraphScalarValue>,
        executor: &Executor<Self::Context, WundergraphScalarValue>,
    ) -> ExecutionResult<WundergraphScalarValue> {
        match *self {
            HasMany::NotLoaded => Err(FieldError::new("HasMany relation not loaded", Value::Null)),
            HasMany::Items(ref i) => i.resolve_field(info, field_name, arguments, executor),
        }
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&[Selection<WundergraphScalarValue>]>,
        executor: &Executor<Self::Context, WundergraphScalarValue>,
    ) -> ExecutionResult<WundergraphScalarValue> {
        match *self {
            HasMany::NotLoaded => Err(FieldError::new("HasMany relation not loaded", Value::Null)),
            HasMany::Items(ref i) => i.resolve_into_type(info, type_name, selection_set, executor),
        }
    }

    fn concrete_type_name(&self, context: &Self::Context, info: &Self::TypeInfo) -> String {
        match *self {
            HasMany::NotLoaded => unreachable!(),
            HasMany::Items(ref i) => i.concrete_type_name(context, info),
        }
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<WundergraphScalarValue>]>,
        executor: &Executor<Self::Context, WundergraphScalarValue>,
    ) -> Value<WundergraphScalarValue> {
        match *self {
            HasMany::NotLoaded => unreachable!(),
            HasMany::Items(ref i) => i.resolve(info, selection_set, executor),
        }
    }
}
