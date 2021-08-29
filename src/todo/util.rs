use super::data::*;
use rusqlite::{params, Connection, Row, NO_PARAMS};

use crate::internal_error::{InternalError, InternalResult};

use std::collections::HashMap;

pub fn get_todos(
    db_connection: &Connection,
) -> InternalResult<Vec<(TodoCategoryID, TodoCategory)>> {
    let mut categories_statement =
        db_connection.prepare("SELECT rowid, name, order FROM todo_categories")?;

    let mut todo_category_map: HashMap<TodoCategoryID, TodoCategory> = HashMap::new();

    let category_rows = categories_statement.query_map(NO_PARAMS, |row| {
        let category_id = row.get::<usize, TodoCategoryID>(0)?;
        let name = row.get::<usize, String>(1)?;
        let order = row.get::<usize, i64>(2)?;

        Ok((
            category_id,
            TodoCategory {
                name,
                order: order as usize,
                todos: vec![],
            },
        ))
    })?;

    for row_result in category_rows {
        let (category_id, category) = row_result?;

        todo_category_map.insert(category_id, category);
    }

    let mut todo_map: HashMap<TodoID, (TodoCategoryID, Todo)> = HashMap::new();

    let mut todos_statement =
        db_connection.prepare("SELECT category_id, rowid, content, order FROM todo_tasks")?;

    let todo_rows = todos_statement.query_map(NO_PARAMS, |row| {
        let category_id = row.get::<usize, TodoCategoryID>(0)?;
        let todo_id = row.get::<usize, TodoID>(1)?;
        let content = row.get::<usize, String>(2)?;
        let order = row.get::<usize, i64>(3)?;

        Ok((
            todo_id,
            category_id,
            Todo {
                content,
                order: order as usize,
                goals: vec![],
            },
        ))
    })?;

    for row_result in todo_rows {
        let (todo_id, category_id, todo) = row_result?;

        todo_map.insert(todo_id, (category_id, todo));
    }

    let mut todo_due_dates_statement =
        db_connection.prepare("SELECT task_id, date FROM todo_due_dates")?;

    let todo_due_dates_rows = todo_due_dates_statement.query_map(NO_PARAMS, |row| {
        let task_id = row.get::<usize, TodoID>(0)?;
        let date = row.get::<usize, String>(1)?;

        Ok((task_id, DueDate { date }))
    })?;

    for row_result in todo_due_dates_rows {
        let (todo_id, due_date) = row_result?;

        if let Some((_, todo)) = todo_map.get_mut(&todo_id) {
            todo.goals.push(TodoGoal::DueDate(due_date));
        }
    }

    let mut todo_progress_statement =
        db_connection.prepare("SELECT task_id, progress, max_progress FROM todo_progress")?;

    let todo_progress_rows = todo_progress_statement.query_map(NO_PARAMS, |row| {
        let task_id = row.get::<usize, TodoID>(0)?;
        let progress = row.get::<usize, i64>(1)?;
        let max_progress = row.get::<usize, i64>(1)?;

        Ok((
            task_id,
            Progress {
                progress: progress as u64,
                max_progress: max_progress as u64,
            },
        ))
    })?;

    for row_result in todo_progress_rows {
        let (task_id, progress) = row_result?;

        if let Some((_, todo)) = todo_map.get_mut(&task_id) {
            todo.goals.push(TodoGoal::Progress(progress));
        }
    }

    let mut todo_dependencies_statement =
        db_connection.prepare("SELECT parent_task_id, child_task_id FROM todo_dependencies")?;

    let todo_dependenies_rows = todo_dependencies_statement.query_map(NO_PARAMS, |row| {
        let parent_task_id = row.get::<usize, TodoID>(0)?;
        let child_task_id = row.get::<usize, TodoID>(0)?;

        Ok((parent_task_id, child_task_id))
    })?;

    for row_result in todo_dependenies_rows {
        let (parent_task_id, child_task_id) = row_result?;

        if let Some((_, todo)) = todo_map.get_mut(&parent_task_id) {
            todo.goals.push(TodoGoal::Dependency(child_task_id));
        }
    }

    for (todo_id, (category_id, todo)) in todo_map.iter() {
        if let Some(category) = todo_category_map.get_mut(&category_id) {
            category.todos.push((*todo_id, todo.clone()));
        }
    }

    Ok(todo_category_map
        .iter()
        .map(|(id, todo)| (*id, todo.clone()))
        .collect::<Vec<(TodoCategoryID, TodoCategory)>>())
}
