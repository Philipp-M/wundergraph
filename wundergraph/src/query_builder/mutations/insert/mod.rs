use crate::query_builder::selection::offset::ApplyOffset;
use crate::query_builder::selection::LoadingHandler;
use crate::query_builder::selection::order::BuildOrder;
use crate::query_builder::selection::SqlTypeOfPlaceholder;
use crate::query_builder::selection::select::BuildSelect;
use crate::scalar::WundergraphScalarValue;
use diesel::backend::Backend;
use diesel::query_builder::QueryFragment;
use diesel::QuerySource;
use juniper::{Arguments, ExecutionResult, Executor, FieldError, FromInputValue, Selection, Value};

#[cfg(feature = "postgres")]
mod pg;

#[cfg(feature = "sqlite")]
mod sqlite;

pub fn handle_insert<DB, I, R, Ctx>(
    selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
    executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    arguments: &Arguments<'_, WundergraphScalarValue>,
    field_name: &'static str,
) -> ExecutionResult<WundergraphScalarValue>
where
    R: LoadingHandler<DB, Ctx>,
    R::Table: HandleInsert<R, I, DB, Ctx> + 'static,
    DB: Backend + ApplyOffset + 'static,
    DB::QueryBuilder: Default,
    R::Columns: BuildOrder<R::Table, DB>
        + BuildSelect<
            R::Table,
            DB,
            SqlTypeOfPlaceholder<R::FieldList, DB, R::PrimaryKeyIndex, R::Table, Ctx>,
        >,
    <R::Table as QuerySource>::FromClause: QueryFragment<DB>,
    I: FromInputValue<WundergraphScalarValue>,
{
    if let Some(n) = arguments.get::<I>(field_name) {
        <R::Table as HandleInsert<_, _, _, _>>::handle_insert(selection, executor, n)
    } else {
        let msg = format!("Missing argument {}", field_name);
        Err(FieldError::new(&msg, Value::Null))
    }
}

pub fn handle_batch_insert<DB, I, R, Ctx>(
    selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
    executor: &Executor<'_, Ctx, WundergraphScalarValue>,
    arguments: &Arguments<'_, WundergraphScalarValue>,
    field_name: &'static str,
) -> ExecutionResult<WundergraphScalarValue>
where
    R: LoadingHandler<DB, Ctx>,
    R::Table: HandleBatchInsert<R, I, DB, Ctx> + 'static,
    DB: Backend + ApplyOffset + 'static,
    DB::QueryBuilder: Default,
    R::Columns: BuildOrder<R::Table, DB>
        + BuildSelect<
            R::Table,
            DB,
            SqlTypeOfPlaceholder<R::FieldList, DB, R::PrimaryKeyIndex, R::Table, Ctx>,
        >,
    <R::Table as QuerySource>::FromClause: QueryFragment<DB>,
    I: FromInputValue<WundergraphScalarValue>,
{
    if let Some(n) = arguments.get::<Vec<I>>(field_name) {
        <R::Table as HandleBatchInsert<_, _, _, _>>::handle_batch_insert(selection, executor, n)
    } else {
        let msg = format!("Missing argument {}", field_name);
        Err(FieldError::new(&msg, Value::Null))
    }
}

pub trait HandleInsert<L, I, DB, Ctx> {
    fn handle_insert(
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        insertable: I,
    ) -> ExecutionResult<WundergraphScalarValue>;
}

pub trait HandleBatchInsert<L, I, DB, Ctx> {
    fn handle_batch_insert(
        selection: Option<&'_ [Selection<'_, WundergraphScalarValue>]>,
        executor: &Executor<'_, Ctx, WundergraphScalarValue>,
        insertable: Vec<I>,
    ) -> ExecutionResult<WundergraphScalarValue>;
}
