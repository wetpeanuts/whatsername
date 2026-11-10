pub struct CloseEvent {
    pub status_code: i32,
}

impl CloseEvent {
    pub fn new(status: i32) -> Self {
        Self {
            status_code: status,
        }
    }
}
