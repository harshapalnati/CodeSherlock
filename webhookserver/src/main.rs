use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde_json::Value;
use reqwest::Client;
use std::env;
use dotenv::dotenv;

#[post("/webhook")]
async fn github_webhook(payload: web::Json<Value>) -> impl Responder {
    println!("✅ Received GitHub Webhook Event");

    if let Some(action) = payload["action"].as_str() {
        if action == "opened" || action == "synchronize" {
            if let Some(pr_number) = payload["pull_request"]["number"].as_i64() {
                if let Some(repo) = payload["repository"]["full_name"].as_str() {
                    //println!("📌 Processing PR #{} in {}", pr_number, repo);

                    // Fetch PR Code Changes & Post Comments
                    match analyze_pr_and_comment(repo, pr_number).await {
                        Ok(_) => println!("✅ AI Comments Successfully Posted!"),
                        Err(e) => println!("❌ Failed to Post AI Comments: {}", e),
                    }
                }
            }
        }
    }

    HttpResponse::Ok().body("Webhook received")
}

 async fn analyze_commit(&self, request: Request<CommitRequest>) -> Result<Response<CommitResponse>, Status> {
        let req = request.into_inner();
        println!("🔄 Processing Commit for Repository: {}", req.repository);

        match generate_commit_analysis(&req.changed_files).await {
            Ok((commit_message, docstrings, test_cases)) => {
                println!("✅ AI Generated Commit Message: {}", commit_message);

                let response = CommitResponse {
                    commit_message,
                    docstrings,
                    test_cases,
                };

                Ok(Response::new(response))
            }
            Err(e) => {
                eprintln!("❌ Failed to generate commit analysis: {}", e);
                Err(Status::internal("Commit analysis failed"))
            }
        }
    }
}

// Function to fetch PR file changes & post comments
async fn analyze_pr_and_comment(repo: &str, pr_number: i64) -> Result<(), reqwest::Error> {
    let github_token = env::var("GITHUB_TOKEN").expect("⚠️ GITHUB_TOKEN not set in .env");
    let client = Client::new();

    // Step 1: Get PR Files
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
                println!("📄 File: {}", filename);

                // Step 2: AI Code Review (Simulated for now)
                let ai_comment = format!("💡 AI Suggestion: Consider improving the logic in `{}`", filename);

                // Step 3: Post AI Comment
                post_pr_comment(repo, pr_number, filename, ai_comment).await?;
            }
        }
    }

    Ok(())
}

// Function to post AI-generated comments on PR
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
        .header("Accept", "application/vnd.github.v3+json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    println!("🔍 GitHub API Response: Status: {}, Body: {}", status, response_text);

    if status != reqwest::StatusCode::CREATED {
        println!("❌ GitHub API Error: {}", response_text);
    }

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
