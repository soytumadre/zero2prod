use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No more expectation, test is focused on another aspect of app behavior
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_the_confirmation_link_confirms_a_subscriber() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No more expectation, test is focused on another aspect of app behavior
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status() // So we return an error if non-200 (I think)
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn confirm_fails_if_there_is_a_fatal_subscription_tokens_database_error() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No more expectation, test is focused on another aspect of app behavior
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let response = reqwest::get(confirmation_links.html)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn confirm_fails_if_there_is_a_fatal_subscriptions_database_error() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No more expectation, test is focused on another aspect of app behavior
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN status;",)
        .execute(&test_app.db_pool)
        .await
        .unwrap();

    let response = reqwest::get(confirmation_links.html)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn confirm_returns_401_with_invalid_subscription_token() {
    let test_app = spawn_app().await;

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // No more expectation, test is focused on another aspect of app behavior
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    // Assert
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    let mut html_link = confirmation_links.html;
    html_link.set_query(Some("subscription_token=my-invalid-token"));

    let response = reqwest::get(html_link)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 401);
}
