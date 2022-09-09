use futures_util::StreamExt;
use twilight_gateway::{Event, Intents, Shard};
use twilight_http::Client;
use twilight_model::http::attachment::Attachment;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("TOKEN")?;

    let (shard, mut events) = Shard::new(token.clone(), Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT);
    let http = Client::new(token.clone());

    shard.start().await?;
    let me = http.current_user().exec().await.unwrap().model().await.unwrap();
    println!("{}#{} {}", me.name, me.discriminator, me.id);


    let valid_url1 = regex::Regex::new(r"https?://www\.tiktok\.com/(?:embed|@(?P<user_id>[\w\.-]+)/video)/(?P<id>\d+)").unwrap();
    let valid_url2 = regex::Regex::new(r"https?://(?:vm|vt)\.tiktok\.com/(?P<id>\w+)").unwrap();

    while let Some(event) = events.next().await {
        if let Event::MessageCreate(message) = event {

            if valid_url1.is_match(message.content.as_str()) || valid_url2.is_match(message.content.as_str()) {
                http.create_typing_trigger(message.channel_id)
                    .exec().await.ok();

                let matches = valid_url1.find_iter(message.content.as_str())
                    .chain(valid_url2.find_iter(message.content.as_str()));

                let mut attachments = vec![];
                for (i, url) in matches.enumerate() {
                    match tokio::spawn(get_video(String::from(url.as_str()), i)).await {
                        Ok(attachment) => attachments.push(attachment),
                        Err(err) => eprintln!("{}", err)
                    }
                }

                http.create_message(message.channel_id)
                    .reply(message.id)
                    .attachments(attachments.as_slice()).unwrap()
                    .exec().await?;
            }

        }
    }

    Err(anyhow::anyhow!("How did we get here?"))
}

async fn get_video(url: String, attachment_idx: usize) -> Attachment {
    let output = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
        .args(["/C", "yt-dlp"])
        .args([url.as_str(), "-f", "best*[vcodec=h264]", "-o", "-"])
        .output().await.unwrap();

    if !output.status.success() {
        panic!("Process excited unsuccessfully: {:?}\n{:?}", output.status.code(), String::from_utf8(output.stderr));
    }

    Attachment::from_bytes(String::from("video.mp4"), output.stdout, attachment_idx as u64)
}