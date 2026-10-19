use crate::ContentType;


pub struct Classification {
    content: ContentType,
    field: String,
    inspected: usize,
    valid: usize,
}
