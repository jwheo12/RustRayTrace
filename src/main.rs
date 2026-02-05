mod config;
mod books;

fn normalize_book_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
}

fn main() {
    eprintln!("Rayon threads: {}", rayon::current_num_threads());

    let mut args = std::env::args().skip(1);
    let book_arg = args.next().unwrap_or_else(|| "in_one_weekend".to_string());
    let scene = args.next().and_then(|arg| arg.parse::<i32>().ok());

    match normalize_book_name(&book_arg).as_str() {
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
