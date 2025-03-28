use async_openai::{
    types::{CreateCompletionRequestArgs, CreateImageRequestArgs, ImageResponseFormat, ImageSize, RequiredAction},
    Client, 
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    create_chat().await;
    Ok(())
}
async fn create_image() -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let request = CreateImageRequestArgs::default()
        .prompt("cats on sofa and carpet in living room")
        .n(2)
        .response_format(ImageResponseFormat::Url)
        .size(ImageSize::S256x256)
        .user("async-openai")
        .build()?;

    let response = client.images().create(request).await?;

    // Download and save images to ./data directory.
    // Each url is downloaded and saved in dedicated Tokio task.
    // Directory is created if it doesn't exist.
    let paths = response.save("./data").await?;

    paths
        .iter()
        .for_each(|path| println!("Image file path: {}", path.display()));

    Ok(())
}

async fn create_chat() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let request = CreateCompletionRequestArgs::default()
        .model("gpt-3.5-turbo-instruct")
        .prompt("Translate the following English text to French: 'Hello, how are you?'")
        .build()?;
    let response = client.completions().create(request).await.unwrap();
    println!("Response: {}", response.choices[0].text);
    Ok(())
}
