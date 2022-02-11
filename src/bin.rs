use conn::*;

fn main() {
    let connection = Connection::new("http://localhost:1337");
    println!("{:#?}", connection.test_connection());
}
