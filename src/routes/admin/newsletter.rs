use actix_web::HttpResponse;

pub async fn get_newsletter_page() -> HttpResponse {
    let body = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>New newsletter issue</title>
</head>
<body>
    <form name="newIssue" action="/admin/newsletters" method="post">
        <label>
            Title
            <input type="text" placeholder="Issue Title" name="title">
        </label>
        <br />
        <label>
            Text content
            <input type="text" placeholder="Text content" name="text_content">
        </label>
        <br />
        <label>
            HTML content
            <input type="text" placeholder="HTML Content" name="html_content">
        </label>
        <br />
        <button type="submit">Send</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>
    "#;

    HttpResponse::Ok().body(body)
}
