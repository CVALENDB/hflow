use std::sync::{Arc, Mutex};
use colored::{Colorize, CustomColor};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

const SPINNER_FRAMES: [&str; 4] = ["—", "\\", "|", "/"];

/// Represents the possible states of an individual execution unit.
#[derive(Clone, Copy, PartialEq)]
pub enum ExecutionStatus {
    InProgress,
    Completed,
    Failed,
}

/// The smallest unit of work, containing logic and a display loop.
pub struct ExecutionUnit {
    status: Arc<Mutex<ExecutionStatus>>,
    description: Arc<String>,
    total_groups: Arc<i32>,
    current_group_idx: Arc<i32>,
    action: Option<Box<dyn FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static>>,
}

impl ExecutionUnit {
    /// Creates a new execution unit with a description and the closure to execute.
    pub fn new<F>(description: String, action: F) -> Self
    where
        F: FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static,
    {
        Self {
            status: Arc::new(Mutex::new(ExecutionStatus::InProgress)),
            description: Arc::new(description),
            total_groups: Arc::new(0),
            current_group_idx: Arc::new(0),
            action: Some(Box::new(action)),
        }
    }

    pub fn set_total_groups(&mut self, total: i32) {
        self.total_groups = Arc::new(total);
    }

    pub fn set_group_index(&mut self, index: i32) {
        self.current_group_idx = Arc::new(index);
    }

    /// Handles the visual feedback (spinner and status) in the terminal.
    fn display_progress(&mut self) {
        let mut spinner = SPINNER_FRAMES.iter().cycle();
        loop {
            let current_status = {
                let guard = self.status.lock().unwrap();
                *guard
            };

            match current_status {
                ExecutionStatus::InProgress => {
                    let output = format!(
                        "\r\x1b[2K[{}/{}] {} {}",
                        self.current_group_idx, self.total_groups, self.description, spinner.next().unwrap()
                    );
                    print!("{}", output.custom_color(CustomColor::new(121, 115, 118)));
                    io::stdout().flush().unwrap();
                }
                ExecutionStatus::Completed => {
                    let output = format!("[{}/{}] {} ✔", self.current_group_idx, self.total_groups, self.description);
                    print!("\r\x1b[2K");
                    println!("{}", output.green());
                    break;
                }
                ExecutionStatus::Failed => {
                    let output = format!("[{}/{}] {} ✘", self.current_group_idx, self.total_groups, self.description);
                    print!("\r\x1b[2K");
                    println!("{}", output.red());
                    std::process::exit(1);
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Spawns the logic thread and starts the display loop.
    pub fn execute(&mut self) {
        if let Some(logic) = self.action.take() {
            let status_ptr = Arc::clone(&self.status);
            thread::spawn(move || {
                logic(status_ptr);
            });
        }
        self.display_progress();
    }
}

/// A logical group of execution units that will be processed sequentially.
pub struct TaskGroup {
    units: Vec<ExecutionUnit>,
}

impl TaskGroup {
    pub fn new() -> Self {
        Self { units: Vec::new() }
    }

    pub fn add_unit(&mut self, unit: ExecutionUnit) {
        self.units.push(unit);
    }

    /// Executes all units within the group one after another.
    pub fn run(&mut self, total_groups: i32, current_idx: i32) {
        for unit in &mut self.units {
            unit.set_group_index(current_idx);
            unit.set_total_groups(total_groups);
            unit.execute();
        }
    }
}

/// The main manager that orchestrates multiple task groups.
pub struct ProgressManager {
    groups: Vec<TaskGroup>,
}

impl ProgressManager {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_group(&mut self, group: TaskGroup) {
        self.groups.push(group);
    }

    /// Starts the execution of all registered task groups.
    pub fn start(&mut self) {
        let total = self.groups.len() as i32;
        for (idx, group) in self.groups.iter_mut().enumerate() {
            group.run(total, (idx + 1) as i32);
        }
    }
}