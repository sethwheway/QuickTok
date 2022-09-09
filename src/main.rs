use std::sync::Arc;

use futures_util::StreamExt;
use twilight_gateway::{Event, Intents, Shard};
use twilight_http::Client;
use twilight_model::{
    gateway::payload::incoming::MessageCreate as MessageCreatePayload,
    http::attachment::Attachment,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("TOKEN")?;

    let (shard, mut events) = Shard::new(token.clone(), Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT);
    let http = Arc::new(Client::new(token.clone()));

    shard.start().await?;
    let me = http.current_user().exec().await.unwrap().model().await.unwrap();
    println!("{}#{} {}", me.name, me.discriminator, me.id);


    let valid_url = regex::Regex::new(r"https?://www\.tiktok\.com/(?:embed|@(?P<user_id>[\w\.-]+)/video)/(?P<id>\d+)").unwrap();

    while let Some(event) = events.next().await {
        if let Event::MessageCreate(message) = event {

            if let Some(url) = valid_url.find(message.content.as_str()) {
                tokio::spawn(handle(http.clone(), String::from(url.as_str()), message)).await
                    .unwrap_or_else(|err| eprint!("{}", err));
            }
        }
    }

    Err(anyhow::anyhow!("How did we get here?"))
}

async fn handle(http: Arc<Client>, url: String, message: Box<MessageCreatePayload>) {
    http.create_typing_trigger(message.channel_id)
        .exec().await.unwrap();

    let output = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
        .args(["/C", "yt-dlp"])
        .args([url.as_str(), "-f", "best*[vcodec=h264]", "-o", "-"])
        .output().await.unwrap();

    if !output.status.success() {
        panic!("Process excited unsuccessfully: {:?}\n{:?}", output.status.code(), String::from_utf8(output.stderr));
    }

    let video = output.stdout;
    http.create_message(message.channel_id)
        .reply(message.id)
        .attachments(&[Attachment::from_bytes(String::from("video.mp4"), video, 0)]).unwrap()
        .exec().await.unwrap();
}