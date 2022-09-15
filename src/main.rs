use serenity::{async_trait, http::typing::Typing, model::prelude::*, prelude::*};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let mut client = Client::builder
        (token, GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT)
        .event_handler(Handler)
        .await?;

    client.start().await?;


    Err(anyhow::anyhow!("How did we get here?"))
}


lazy_static::lazy_static! {
    static ref VALID_URLS: [regex::Regex; 2] = [
        regex::Regex::new(r"https?://www\.tiktok\.com/(?:embed|@(?P<user_id>[\w\.-]+)/video)/(?P<id>\d+)").unwrap(),
        regex::Regex::new(r"https?://(?:vm|vt)\.tiktok\.com/(?P<id>\w+)").unwrap()
    ];
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{}#{} {}\nGuilds: {}", ready.user.name, ready.user.discriminator, ready.user.id, ready.guilds.len());
    }

    async fn message(&self, ctx: Context, message: Message) {
        let content = message.content.as_str();
        if !(VALID_URLS[0].is_match(content) || VALID_URLS[1].is_match(content)) { return; }

        let typing = Typing::start(ctx.http.clone(), message.channel_id.0);

        let matches = VALID_URLS.iter().flat_map(|re| re.find_iter(content));
        let handles = matches.map(|url| tokio::spawn(get_video(String::from(url.as_str()))));

        let mut errored = 0;
        let mut videos = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(video) => videos.push(video),
                Err(err) => {
                    eprint!("{}", err);
                    errored += 1;
                }
            }
        }

        let mut content = String::from("Sorry! ");
        if errored > 0 {
            content += format!(
                "Something went wrong with {} video{}.",
                errored, if errored > 1 { "s" } else { "" }
            ).as_str();
        }

        message.channel_id.send_message(ctx.http, |m| {
            m.reference_message(&message);
            for video in videos {
                m.add_file(AttachmentType::Bytes {
                    data: std::borrow::Cow::from(video),
                    filename: String::from("video.mp4"),
                });
            }
            if errored > 0 { m.content(content); }
            m
        }).await.unwrap();


        if let Ok(typing) = typing { typing.stop(); }
    }
}

async fn get_video(url: String) -> Vec<u8> {
    let output = tokio::process::Command::new("./yt-dlp")
        .args([url.as_str(), "-f", "best*[vcodec=h264][filesize<8M]", "-o", "-"])
        .output().await.unwrap();

    if !output.status.success() {
        panic!(
            "Process excited unsuccessfully: {:?}\n{:?}",
            output.status.code(), String::from_utf8(output.stderr)
        )
    }

    output.stdout
}
