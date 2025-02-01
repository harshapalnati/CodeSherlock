use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde_json::Value;
use reqwest::Client;
use std::env;
use dotenv::dotenv;

#[post("/webhook")]
async fn github_webhook(payload: web::Json<Value>) -> impl Responder {
   // println!("✅ Received GitHub Webhook Event: {:?}", payload);

    if let Some(action) = payload["action"].as_str() {
        if action == "opened" || action == "synchronize" {
            if let Some(pr_number) = payload["pull_request"]["number"].as_i64() {
                if let Some(repo) = payload["repository"]["full_name"].as_str() {
                     //println!("📌 Processing PR #{} in {}", pr_number, repo);

                    // Fetch PR Code Changes & Analyze with GPT-4
                    match analyze_pr_with_gpt(repo, pr_number).await {
                        Ok(_) => println!("✅ AI Review Comments Posted!"),
                        Err(e) => println!("❌ Failed to Post AI Comments: {}", e),
                    }
                }
            }
        }
    }

    HttpResponse::Ok().body("Webhook received")
}

// Fetch PR changes & analyze with GPT-4
async fn analyze_pr_with_gpt(repo: &str, pr_number: i64) -> Result<(), reqwest::Error> {
    let github_token = env::var("GITHUB_TOKEN").expect("⚠️ GITHUB_TOKEN not set in .env");
    let openai_key = env::var("OPENAI_API_KEY").expect("⚠️ OPENAI_API_KEY not set in .env");
    let client = Client::new();

    // Step 1: Get PR File Changes
    let url = format!("https://api.github.com/repos/{}/pulls/{}/files", repo, pr_number);
    let response = client
        .get(&url)
        .header("Authorization", format!("token {}", github_token))
        .header("User-Agent", "rust-bot")
        .send()
        .await?
        .json::<Value>()
        .await?;

    for file in response.as_array().unwrap_or(&vec![]) {
        if let Some(filename) = file["filename"].as_str() {
            if let Some(patch) = file["patch"].as_str() {
                println!("📄 Analyzing {} with GPT-4...", filename);

                // Step 2: Send to GPT-4 for Code Review
                let ai_comment = get_gpt4_analysis(filename, patch, &openai_key).await?;
                println!("🔍 AI Comment: {}", ai_comment);
                // Step 3: Post AI Comment on GitHub
                post_pr_comment(repo, pr_number, filename, ai_comment).await?;
            }
        }
    }

    Ok(())
}

// Send PR changes to GPT-4 for AI Review
async fn get_gpt4_analysis(filename: &str, code_diff: &str, openai_key: &str) -> Result<String, reqwest::Error> {
    let client = Client::new();

    let prompt = format!(
        "You are an AI code reviewer. Analyze the following GitHub Pull Request change for bugs, security issues, and best practices. Suggest improvements:\n\nFile: {}\nCode Diff:\n{}",
        filename, code_diff
    );

    let payload = serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 200,
        "temperature": 0.7
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?
        .json::<Value>()
        .await?;

    let comment = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No suggestions found.")
        .to_string();

    Ok(comment)
}

// Post AI-generated comments on GitHub PR
async fn post_pr_comment(repo: &str, pr_number: i64, filename: &str, comment: String) -> Result<(), reqwest::Error> {
    let github_token = env::var("GITHUB_TOKEN").expect("⚠️ GITHUB_TOKEN not set in .env");
    let client = Client::new();

    let url = format!("https://api.github.com/repos/{}/issues/{}/comments", repo, pr_number);

    let payload = serde_json::json!({
        "body": format!("🔍 AI Code Review for `{}`:\n{}", filename, comment),
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("token {}", github_token))
        .header("User-Agent", "rust-bot")
        .json(&payload)
        .send()
        .await?;

    println!("✅ AI Review Comment Posted: {:?}", response.text().await?);
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let port = env::var("PORT").unwrap_or("3000".to_string());

    println!("🚀 GitHub Webhook Listener Running on 0.0.0.0:{}", port);

    HttpServer::new(|| App::new().service(github_webhook))
        .bind(format!("0.0.0.0:{}", port))?
        .run()
        .await
}
