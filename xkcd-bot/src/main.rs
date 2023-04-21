use serenity::async_trait;
//use serenity::model::prelude::Deserialize;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::{StandardFramework};

use std::fs::{File, self};
use std::io::prelude::*;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use toml;

use tokio::time::sleep;

#[group]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        if let Some(shard) = ready.shard {
            // Note that array index 0 is 0-indexed, while index 1 is 1-indexed.
            //
            // This may seem unintuitive, but it models Discord's behaviour.
            println!("{} is connected on shard {}/{}!", ready.user.name, shard[0], shard[1]);
        }
    }
    async fn message(&self, ctx: Context, msg: Message)  {
        // have to make sure bot doesn't infinite loop reply to its own messages
        if msg.author.id != ctx.cache.current_user_id() {
            // opening the text file with the names of all the comics
            let namespath = Path::new("./comic_names.txt");
            let display = namespath.display();
            let mut file = match File::open(&namespath) {
                Err(why) => panic!("couldn't open {}: {}", display, why),
                Ok(file) => file,
            };
        
            // Read the file contents into a string, returns `io::Result<usize>`
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Err(why) => panic!("couldn't read {}: {}", display, why),
                Ok(_) => (),
            }

            // process the names so that there is a vector that has the names in the form of another vector with strings, each being 1 word of the name
            let comic_names = s.split("\n");
            let mut matcher: Vec<Vec<String>> = Vec::new();
            for name in comic_names.into_iter() {
                matcher.push(Vec::from_iter(name.split(",").map(str::to_string).into_iter()));
            }

            // now we go through each comic (and which comic it is)
            let mut goods: Vec<(usize, usize)> = Vec::new();
            for (comic, name) in matcher.iter().enumerate() {
                // directly checks if each word in the name is in the message content
                let full_name = name.join(" ");
                let matching = msg.content.to_lowercase().contains(&full_name);
                // if we find a comic that matches, we append it to the list of matching comics with the comic name's length
                if matching {
                    goods.push((comic, full_name.len()));
                }
            }
            // identify the longest matching comic and reply to that one only
            if goods.len() > 0 {
                let best_good = goods.iter().max_by(|a,b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
                if best_good.1 > 1 {        // basically just to check for the comic "i"
                    // now we get the path of the image of the comic we will send and check if it exists
                    let address = format!("images/comic_{}.png", best_good.0+1);
                    let filepath = Path::new(&address);
                    if filepath.exists() {
                        // sending the message and awaiting to make sure it happens
                        let _ = msg.channel_id.send_message(&ctx.http, |m| {
                            m.reference_message(&msg)
                                .content(format!("Speaking of {}, here's something to think about:",bolden(&matcher[best_good.0].join(" "))))
                                .add_file(filepath)
                        }).await;
                    }
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct Secrets {
    bot_token: String,
}

#[tokio::main]
async fn main() {

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix for all commands
        .group(&GENERAL_GROUP);

    // open Secrets.toml
    let filename = "Secrets.toml";
    let file = fs::read_to_string(&filename).unwrap();
    // extract bot token from toml
    let secrets: Secrets = toml::from_str(&file).unwrap();
    let token = secrets.bot_token;
    // set up bot
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    let manager = client.shard_manager.clone();

    // does stuff
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(5)).await;

            let lock = manager.lock().await;
            let shard_runners = lock.runners.lock().await;

            for (id, runner) in shard_runners.iter() {
                println!(
                    "Shard ID {} is {} with a latency of {:?}",
                    id, runner.stage, runner.latency,
                );
            }
        }
    });

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

}


fn bolden(input: &str) -> String {
    format!("**{}**",input)
}
