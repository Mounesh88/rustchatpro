use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

// ... other code ...

fn serve_html(req: &Request<Body>) -> impl IntoResponse {
    let path = format!("static/{{}}", req.uri().path());
    let file_path = Path::new(&path);

    // Attempt to read the file
    let mut file = match File::open(&file_path) {
        Ok(file) => file,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found"),
    };

    // Read the file content
    let mut content = String::new();
    if let Err(_) = file.read_to_string(&mut content) {
        return (StatusCode::NOT_FOUND, "File read error");
    }

    (StatusCode::OK, content)
}