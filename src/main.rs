use bson::{bson, doc, Bson};
use mongodb::{options::FindOptions, Client};
use std::{error::Error, thread, time::Duration};

/// The generic result type for this crate.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    println!("Bot ready and waiting 10 seconds for DB");
    thread::sleep(Duration::from_secs(10));

    // Connect to DB & get handles to user DB and the books collection
    let client = Client::with_uri_str("mongodb://db:27017/")?;
    let db = client.database("users");
    let collection = db.collection("books");

    // Insert some documents into the "users.books" collection
    let docs = vec![
        doc! { "title": "1984", "author": "George Orwell" },
        doc! { "title": "Animal Farm", "author": "George Orwell" },
        doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
    ];
    collection.insert_many(docs, None)?;

    // Query the documents in the collection with a filter and an option
    let filter = doc! { "author": "George Orwell" };
    let find_options =
        FindOptions::builder().sort(doc! { "title": 1 }).build();
    let cursor = collection.find(filter, find_options)?;
    // Iterate over the results of the cursor
    for result in cursor {
        match result {
            Ok(document) => {
                if let Some(title) =
                    document.get("title").and_then(Bson::as_str)
                {
                    println!("title: {}", title);
                } else {
                    println!("no title found");
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
