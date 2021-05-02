use gtk::{Button, Entry, TreeIter, TreeModel, prelude::*};
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
        let model = gtk::TreeStore::new(&[glib::Type::String]);

        let categories = tasks.get_categories();

        for cat in categories.iter() {
            let parent = model.insert_with_values(None, None, &[0], &[cat]);

            for t in tasks.tasks.iter().filter(|t| t.category == cat.as_str()) {
                let _ = model.insert_with_values(Some(&parent),None,&[0], &[&t.formatted(true)]);
            }
        }

        self.tasks_list.set_model(Some(&model));
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
                        let (tf, cat) = get_selected_data(&model, &iter);
                        
                        if tf.is_some() && cat.is_some() {
                            let tasks = tclone.upgrade();
                            match tasks {
                                Some(t) => {
                                    let t = t.borrow();
                                    let task = find_task_in_list(&t.tasks, &cat.unwrap(),&tf.unwrap());
                                    if let Some(t) = task {
                                        clone.update_task(&t);
                                    }
                                    else {
                                        clone.disable_task();
                                    }
                                },
                                None => ()
                            }
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
                    let (tf, cat) = get_selected_data(&model,&iter);

                    if tf.is_some() && cat.is_some() {
                        let tf = tf.unwrap();
                        let cat = cat.unwrap();
                        match tclone.upgrade() {
                            Some(t) => {
                                let mut t = t.borrow_mut();
                                let mut task = find_task_in_list_mut(&mut t.tasks, &cat, &tf).expect("couldnt find the selected task");
                                
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
                                let tf = task.formatted(true);

                                let _ = t.save(&TasksManager::default_path());
                                drop(t);
                                // clone.update_tasks_list(&*t);
                                // im only using TreeStore here so it should work, but if not i will be sad :(
                                let model : gtk::TreeStore = unsafe {
                                    // The model should be a treestore tbh
                                    model.unsafe_cast()
                                };
                                model.set_value(&iter, 0, &tf.to_value());


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
                    let  (tf, cat) = get_selected_data(&model,&iter);

                    if tf.is_some() && cat.is_some() {
                        let tf = tf.unwrap();
                        let cat = cat.unwrap();

                        match tclone.upgrade() {
                            Some(t) => {
                                let mut t = t.borrow_mut();

                                let dia = gtk::MessageDialog::new(
                                    Some(&clone.main_window),
                                    gtk::DialogFlags::DESTROY_WITH_PARENT,
                                    gtk::MessageType::Warning,
                                    gtk::ButtonsType::YesNo,
                                    &format!("Are you sure you want to delete:\n\t{}", tf)
                                );
                                dia.set_title("Remove Task");
                                let res = dia.run();
                                dia.hide();

                                match res {
                                    gtk::ResponseType::Yes => {
                                        // find the task
                                        let task_i = t.tasks.iter()
                                            .enumerate()
                                            .find(|(_,t)| t.formatted(true) == tf && t.category == cat)
                                            .map(|(i,_)| i);
                                        t.remove_task(task_i.unwrap_or(9999));

                                        let model : gtk::TreeStore = unsafe {
                                            model.unsafe_cast()
                                        };
                                        clone.disable_task();
                                        
                                        let _ = t.save(&TasksManager::default_path());
                                        
                                        // Upon calling `model.remove` the `selection changed` closure will be envoked
                                        // which borrows the tasks manager, thus we need to drop beforehand 
                                        drop(t); 
                                        model.remove(&iter);
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
                    // Make sure we dont already have a 'new task'
                    let n;
                    if !t.tasks.iter().any(|t| t.name == "new task") {
                        let new = Task::new("new task");
    
                        t.add_task(new);
                        clone.update_tasks_list(&*t);
                        n = t.tasks.len() -1;
                    }
                    else {
                        n = {
                            let mut res = 0;
                            for (i,t) in t.tasks.iter().enumerate() {
                                if t.name == "new task" {
                                    res = i;
                                    break;
                                }
                            }
                            res
                        };
                    }
                    // Select the new task
                    let model = clone.tasks_list.get_model().expect("couldnt get model?!?");
                    let iter = model.iter_nth_child(None, n as i32);

                    // Why?
                    //      Forcefully dorp 't' early because we borrow it here as mut
                    //      while we want to select(which will enforce another borrow_mut)
                    // Anyway, dropping it early means we are no longer borrow_mut-ing it!
                    drop(t);

                    match iter {
                        Some(t) => {
                            let selector = clone.tasks_list.get_selection();
                            selector.select_iter(&t);
                        }
                        None => {}
                    }


                    
                }
                None => ()
            }
        });

        // Connect row changed(to see if we need to refresh or not)
        let clone = self.clone();
        let tclone = tasks.clone();
        let model = self.tasks_list.get_model().expect("Couldn't get model wtf");
        model.connect_row_changed(move |model,_,iter| {
            let (tf, cat) = get_selected_data(model,iter);
            if tf.is_some() && cat.is_some() {
                let tf = tf.unwrap();
                let cat = cat.unwrap();

                let parent_path = {
                    let mut path = model.get_path(&iter).expect("couldn't get path");
                    let _ = path.up();
                    path
                };
                // Do stuff with the task
                match tclone.upgrade() {
                    Some(manager) => {
                        let manager = manager.borrow();
                        let task = manager.tasks.iter()
                            .find(|t| t.formatted(true) == tf);
                        
                        if task.is_some() {
                            let task = task.unwrap();
                            if task.category != cat {
                                // move the task to a new category
                                let model : gtk::TreeStore = unsafe {
                                    model.clone().unsafe_cast()
                                };
                                let clean_iter = model.get_iter_first().expect("couldnt get iter");
                                let mut wanted_parent = None;
                                loop {
                                    if !model.iter_next(&clean_iter) {
                                        break;
                                    }
                                    let path = model.get_path(&clean_iter).unwrap();
                                    // Check if this is a category
                                    if path.get_depth() == 1 {
                                        let cat : Result<Option<String>,_> = model.get_value(&clean_iter,0).get();
                                        if cat == Ok(Some(task.category.clone())) {
                                            // Get to its children
                                            wanted_parent = Some(clean_iter);
                                            break;
                                            
                                        }
                                    }
                                }
                                match wanted_parent {
                                    Some(new_pos) => {
                                        let _ = model.remove(&iter);
                                        let select = model.insert_with_values(Some(&new_pos), None, &[0],&[&task.formatted(true)]);
                                        clone.tasks_list.get_selection().select_iter(&select);
                                    },
                                    None => {
                                        // we need to create a new category
                                        let cat = &task.category;
                                        let parent = model.insert_with_values(None, None, &[0],&[&cat]);

                                        let _ = model.remove(&iter);
                                        let select = model.insert_with_values(Some(&parent), None, &[0],&[&task.formatted(true)]);
                                        clone.tasks_list.get_selection().select_iter(&select);
                                    }
                                }
                            }
                        }
                    },
                    None => {}
                }
            
                // Check if the parent needs to be removed
                let parent_iter = model.get_iter(&parent_path).expect("couldnt get path");
                if model.iter_n_children(Some(&parent_iter)) == 0 {
                    let model : gtk::TreeStore = unsafe {
                        model.clone().unsafe_cast()
                    };
                    model.remove(&parent_iter);
                }
            }
        });

        model.connect_row_deleted(|model, path| {
            let parent_path = {
                let mut path = path.clone();
                path.up();
                path
            };
            let iter = model.get_iter(&parent_path);
            if let Some(iter) = iter {
                if model.iter_n_children(Some(&iter)) == 0 {
                    let model : gtk::TreeStore = unsafe {
                        model.clone().unsafe_cast()
                    };
                    model.remove(&iter);
                }
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

/// Returns the formatted task and category - (Option(task_formatted),Option<cat>)
fn get_selected_data(model : &TreeModel, iter :&TreeIter) -> (Option<String>,Option<String>) {
    let mut path = model.get_path(&iter).expect("couldnt get path from iter");

    if path.get_depth() > 1 {
        let _ = path.up();
        let cat = model.get_iter(&path).expect("couldnt get category");
        let cat : Result<Option<String>,_> = model.get_value(&cat, 0).get();
        let formatted : Result<Option<String>,_> = model.get_value(&iter,0).get();

        // Make sure we have both a normal task and category
        if formatted.is_ok() && cat.is_ok() {
            return (formatted.unwrap(), cat.unwrap());
        }

    }
    (None,None)
}

fn find_task_in_list<'a>(task_list : &'a Vec<Task>, cat : &str, formatted_task : &str) -> Option<&'a Task> {
    task_list.iter().find(|t| t.category == cat && t.formatted(true) == formatted_task)
}
fn find_task_in_list_mut<'a>(task_list : &'a mut Vec<Task>, cat : &str, formatted_task : &str) -> Option<&'a mut Task> {
    task_list.iter_mut().find(|t| t.category == cat && t.formatted(true) == formatted_task)
}