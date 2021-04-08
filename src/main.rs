use gtk::{Button, Entry, prelude::*};
use cool_organizer::*;
use std::rc::{Rc,Weak};
use std::cell::RefCell;

fn main() {
    gtk::init().expect("couldnt initialize gtk!");

    

    let glade_src = include_str!("../app.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let ui = get_layout_from_builder(&builder);

    ui.main_window.set_title("Cool Organizer's Beautiful Interface");

    let tasks = TasksManager::load(&TasksManager::default_path());
    let tasks = Rc::new(RefCell::new(tasks));

    ui.initialize(Rc::clone(&tasks));

    ui.main_window.connect_destroy(|_| {
        gtk::main_quit();
    });
    ui.main_window.show_all();
    gtk::main();
}

#[derive(Clone)]
struct UILayout {
    // General stuff
    main_window : gtk::Window,
    save : Button,
    new : Button,
    delete : Button,
    remove_done : Button,
    tasks_list : gtk::TreeView,
    task_grid : gtk::Grid,
    // Task specific
    name : Entry,
    category : Entry,
    sub_cat : Entry,
    due : gtk::Switch,
    date : gtk::Calendar,
    done : gtk::CheckButton,
    prio : gtk::SpinButton,
}
impl UILayout {
    fn initialize(&self, tasks : Rc<RefCell<TasksManager>>) {
        // Disable the task window
        self.disable_task();



        self.update_tasks_list(&tasks.borrow());
        let renderer = gtk::CellRendererText::new();
        let col = gtk::TreeViewColumn::new();
        
        col.pack_start(&renderer, true);
        col.add_attribute(&renderer, "text", 0);

        self.tasks_list.append_column(&col);
        
        self.clone().connect_ui(Rc::downgrade(&tasks));

    }

    fn update_tasks_list(&self, tasks : &TasksManager) {
        let list = gtk::ListStore::new(&[glib::Type::String]);

        for task in tasks.tasks.iter() {
            list.set(&list.append(), &[0], &[&task.formatted(true)]);
        }
        self.tasks_list.set_model(Some(&list));
    }

    fn disable_task(&self) {
        self.task_grid.set_sensitive(false);
    }

    fn update_task(&self, task : &Task) {
        self.task_grid.set_sensitive(true);

        self.name.set_text(&task.name);
        self.category.set_text(&task.category);
        self.sub_cat.set_text(&task.sub_category);
        self.due.set_active(task.due.is_some());

        if let Some(date) = task.due {
            let date = date.to_localdate().expect("error converting date");
            
            self.date.select_month(date.month() as u32 -1, date.year() as u32);
            self.date.select_day(date.day() as u32);
        }

        self.done.set_active(task.done);
        self.prio.set_value(task.priority as f64);
    }

    fn connect_ui(self, tasks : Weak<RefCell<TasksManager>>) {
        // Connect task select
        let clone = self.clone();
        let tclone = tasks.clone();
        self.tasks_list.get_selection()
            .connect_changed(move |ts| {
                let task = ts.get_selected();

                match task {
                    Some((model,iter)) => {
                        let path = model.get_path(&iter).expect("couldnt get path from iter");
                        let row = *path.get_indices().first().unwrap_or(&-1);

                        if row == -1 {
                            clone.disable_task();
                        }
                        else {
                            let tasks = tclone.upgrade();
                            match tasks {
                                Some(t) => clone.update_task(&t.borrow().tasks[row as usize]),
                                None => ()
                            };
                        }
                    },
                    None => {
                        clone.disable_task();
                    }
                }
            });
        // Connect date being disabled
        let date = self.date.clone();
        self.due.connect_changed_active(move |c| {
            date.set_sensitive(c.get_active());
        });
        
        // Connect save
        let clone = self.clone();
        let tclone = tasks.clone();
        self.save.connect_clicked(move |_| {
            let selector = clone.tasks_list.get_selection();
            match selector.get_selected() {
                Some((model, iter)) => {
                    let path = model.get_path(&iter).expect("couldnt get path");
                    let row = *path.get_indices().first().unwrap_or(&-1);

                    if row != -1 {
                        match tclone.upgrade() {
                            Some(t) => {
                                let mut t = t.borrow_mut();
                                let mut task = &mut t.tasks[row as usize];
                                
                                task.name = clone.name.get_text().into();
                                task.category = clone.category.get_text().into();
                                task.sub_category = clone.sub_cat.get_text().into();
                                
                                let due = if clone.due.get_active() {
                                    let (year, month, day) = clone.date.get_date();
                                    let month = Date::month_from_int(month as i32 + 1);
                                    let date = LocalDate::ymd(year as i64, month, day as i8).expect("Couldn't parse date...");
                                    Some(Date::from(date))
                                }
                                else {
                                    None
                                };

                                task.due = due;
                                task.done = clone.done.get_active();

                                task.priority = clone.prio.get_value() as u8;

                                clone.update_tasks_list(&*t);
                                let _ = t.save(&TasksManager::default_path());
                            }
                            None => {}
                        }
                    }
                },
                None => ()
            }
        });

        // Connect delete task
        let clone = self.clone();
        let tclone = tasks.clone();
        self.delete.connect_clicked(move |_| {
            let selector = clone.tasks_list.get_selection();
            match selector.get_selected() {
                Some((model, iter)) => {
                    let path = model.get_path(&iter).expect("couldnt get path");
                    let row = *path.get_indices().first().unwrap_or(&-1);

                    if row != -1 {
                        match tclone.upgrade() {
                            Some(t) => {
                                let mut t = t.borrow_mut();

                                let task_string = t.tasks[row as usize].formatted(true);

                                let dia = gtk::MessageDialog::new(
                                    Some(&clone.main_window),
                                    gtk::DialogFlags::DESTROY_WITH_PARENT,
                                    gtk::MessageType::Warning,
                                    gtk::ButtonsType::YesNo,
                                    &format!("Are you sure you want to delete:\n\t{}", task_string)
                                );
                                dia.set_title("Remove Task");
                                let res = dia.run();
                                dia.hide();

                                match res {
                                    gtk::ResponseType::Yes => {
                                        t.remove_task(row as usize);
                                        clone.update_tasks_list(&*t);
                                        let _ = t.save(&TasksManager::default_path());
                                    }
                                    _ => ()

                                }

                            }
                            None => ()
                        }
                    
                    }

                }
                None => ()
            }

        });

        // Connect remove done
        let clone = self.clone();
        let tclone = tasks.clone();
        self.remove_done.connect_clicked(move |_| {
            let dia = gtk::MessageDialog::new(
                Some(&clone.main_window),
                gtk::DialogFlags::DESTROY_WITH_PARENT,
                gtk::MessageType::Warning,
                gtk::ButtonsType::YesNo,
                "Are you sure you want to remove done?"
            );
            dia.set_title("Remove Done");
            let res = dia.run();
            dia.hide();

            if res == gtk::ResponseType::Yes {
                match tclone.upgrade() {
                    Some(t) => {
                        let mut t = t.borrow_mut();
                        t.remove_done();

                        clone.update_tasks_list(&*t);
                        let _ = t.save(&TasksManager::default_path());
                    }
                    None => ()
                }
            }
        });

        // Connect new task button
        let clone = self.clone();
        let tclone = tasks.clone();
        self.new.connect_clicked(move |_| {
            match tclone.upgrade() {
                Some(t) => {
                    let mut t = t.borrow_mut();
                    let new = Task::new("new task");

                    t.add_task(new);
                    clone.update_tasks_list(&*t);
                }
                None => ()
            }
        });
    }
}

fn get_layout_from_builder(builder : &gtk::Builder) -> UILayout {
    UILayout {
        main_window : builder.get_object("main_window").expect("main_window is missing"),
        save : builder.get_object("btn_save").expect("btn_save is missing"),
        new : builder.get_object("btn_new").expect("btn_new is missing"),
        delete : builder.get_object("btn_delete").expect("btn_delete is missing"),
        remove_done : builder.get_object("btn_rmdone").expect("btn_rmdone is missing"),
        tasks_list : builder.get_object("tasks_list").expect("tasks_list is missing"),
        task_grid : builder.get_object("task_grid").expect("task_grid is missing"),
        name : builder.get_object("task_name").expect("task_name is missing"),
        category : builder.get_object("task_cat").expect("task_cat is missing"),
        sub_cat : builder.get_object("task_sub").expect("task_sub is missing"),
        due : builder.get_object("task_due").expect("task_due is missing"),
        date : builder.get_object("task_date").expect("task_date is  missing"),
        done : builder.get_object("task_done").expect("task_done is missing"),
        prio : builder.get_object("task_prio").expect("task_prio is missing"),
    }
}
