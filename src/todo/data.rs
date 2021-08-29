pub type TodoID = i64;
pub type TodoCategoryID = i64;

#[derive(Clone)]
pub struct Todo {
    pub content: String,
    pub order: usize,
    pub goals: Vec<TodoGoal>,
}

#[derive(Clone)]
pub struct DueDate {
    pub date: String,
}

#[derive(Clone, Copy)]
pub struct Progress {
    pub progress: u64,
    pub max_progress: u64,
}

#[derive(Clone)]
pub enum TodoGoal {
    DueDate(DueDate),
    Progress(Progress),
    Dependency(TodoID),
}

#[derive(Clone)]
pub struct TodoCategory {
    pub name: String,
    pub order: usize,
    pub todos: Vec<(TodoID, Todo)>,
}
