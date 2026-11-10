use whatsername::tui::engine::App;
use whatsername::tui::view::AppView;

fn main() {
    let mut app = match App::new(|app_handle| Box::new(AppView::new(app_handle))) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("whatsername: {error}");
            std::process::exit(1);
        }
    };

    let result = app.exec();
    drop(app);

    match result {
        Ok(status) => std::process::exit(status),
        Err(error) => {
            eprintln!("whatsername: {error}");
            std::process::exit(1);
        }
    }
}
