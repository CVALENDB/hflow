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
    execute: Option<Box<dyn FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static>>,
    on_failure : Option<Box<dyn FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static>>,
    on_sucess : Option<Box<dyn FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static>>,
}

impl ExecutionUnit {
    /// Creates a new execution unit with a description and the closure to execute.
    pub fn new(description: String) -> Self {

        Self {
            status: Arc::new(Mutex::new(ExecutionStatus::InProgress)),
            description: Arc::new(description),
            total_groups: Arc::new(0),
            current_group_idx: Arc::new(0),
            execute: None,
            on_failure : None,
            on_sucess : None,
        }
    }


    pub fn set_total_groups(&mut self, total: i32) {
        self.total_groups = Arc::new(total);
    }

    pub fn set_group_index(&mut self, index: i32) {
        self.current_group_idx = Arc::new(index);
    }

    ///thirst for the main callback
    pub fn on_execute<F>(mut self, callback: F) -> Self
    where
        F: 'static + FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static,
    {
        self.execute = Some(Box::new(callback));
        self
    }

    ///If it fails, the state calls this action instead of terminating the programme.
    pub fn on_failure<F>(mut self, action: F) -> Self
    where
        F : FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static,
    {
        self.on_failure = Some(Box::new(action));
        self
    }

    ///This function is invoked if the status changes to complete.
    pub fn on_success<F>(mut self, action: F) -> Self
    where
        F : FnOnce(Arc<Mutex<ExecutionStatus>>) + Send + 'static,
    {
        self.on_sucess = Some(Box::new(action));
        self
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
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Registers an action to be executed if the task fails.
    ///
    /// # Important
    ///
    /// The callback **MUST** do one of these two things:
    ///
    /// 1. **Call `std::process::exit(1)`** to terminate the program
    /// 2. **Change the status** to another state (NOT recommended)
    ///
    /// If it does neither, **it will enter an infinite loop**
    /// repeatedly printing the failure message.
    ///
    /// # Correct Example
    ///
    /// ```rust
    /// let mut task = ExecutionUnit::new("Migrate DB", |status| {
    ///     migrate_db().unwrap();
    ///     *status.lock().unwrap() = ExecutionStatus::Completed;
    /// });
    ///
    /// task.set_fail_action(|status| {
    ///     println!("Rollback executed");
    ///     rollback_migration();
    ///     std::process::exit(1);  // ← IMPORTANT
    /// });
    /// ```
    ///
    /// # Incorrect Example (infinite loop)
    ///
    /// ```rust,no_run
    /// task.set_fail_action(|status| {
    ///     println!("This will print infinitely");
    ///     // ← Missing exit(1) here
    /// });
    /// ```
    pub fn execute(&mut self) {

        let status = self.status.clone();
        let on_fail = self.on_failure.take();
        let action = self.execute.take().unwrap();
        let success = self.on_sucess.take();

        let description = self.description.clone();
        let current_idx = self.current_group_idx.clone();
        let total = self.total_groups.clone();

        let handle = thread::spawn(move || {
            action(status.clone());


            let final_status = {
                let guard = status.lock().unwrap();
                *guard
            };

            if final_status == ExecutionStatus::Completed {
                if let Some(callback) = success {
                    callback(status.clone());
                }
            }

            if final_status == ExecutionStatus::Failed {
                if let Some(callback) = on_fail {
                    println!("tenemos fail");
                    callback(status.clone());
                } else {
                    println!("no tenemos fail");
                }
            }
        });


        self.display_progress();


        handle.join().unwrap();


        let final_status = {
            let guard = self.status.lock().unwrap();
            *guard
        };

        if final_status == ExecutionStatus::Failed {
            std::process::exit(1);
        }
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