mod config;
mod books;
mod gpu;

fn normalize_book_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
}

fn main() {
    eprintln!("Rayon threads: {}", rayon::current_num_threads());

    let mut backend = "cpu".to_string();
    let mut positional_args = Vec::new();
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        if arg == "--gpu" {
            backend = "gpu".to_string();
            continue;
        }
        if arg == "--cpu" {
            backend = "cpu".to_string();
            continue;
        }
        if arg == "--backend" {
            if let Some(value) = args.next() {
                backend = value;
            } else {
                eprintln!("--backend expects a value: cpu or gpu");
                return;
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix("--backend=") {
            backend = value.to_string();
            continue;
        }
        positional_args.push(arg);
    }

    let backend = backend.to_lowercase();
    let book_arg = positional_args
        .get(0)
        .cloned()
        .unwrap_or_else(|| "in_one_weekend".to_string());
    let scene = positional_args.get(1).and_then(|arg| arg.parse::<i32>().ok());
    let book_key = normalize_book_name(&book_arg);

    if backend == "gpu" {
        if matches!(book_key.as_str(), "inoneweekend" | "oneweekend" | "weekend") {
            match gpu::render_in_one_weekend() {
                Ok(()) => return,
                Err(err) => {
                    eprintln!("GPU render failed: {err}");
                    eprintln!("Falling back to CPU.");
                }
            }
        } else {
            eprintln!("GPU backend currently supports in_one_weekend only. Falling back to CPU.");
        }
    }

    match book_key.as_str() {
        "inoneweekend" | "oneweekend" | "weekend" => books::in_one_weekend::run(None),
        "thenextweek" | "nextweek" | "next" => books::the_next_week::run(scene),
        "therestofyourlife" | "restofyourlife" | "rest" | "restoflife" => {
            books::the_rest_of_your_life::run(None)
        }
        _ => {
            eprintln!("Usage: cargo run -- <book> [scene]");
            eprintln!("books: in_one_weekend, the_next_week, the_rest_of_your_life");
            eprintln!("example: cargo run -- the_next_week 3");
        }
    }
}
