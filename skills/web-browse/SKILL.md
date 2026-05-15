---
name: web-browse
version: "1.0.0"
description: Browse websites and search the web using the local Playwright browser tool
activation:
  keywords:
    - "browse"
    - "search"
    - "website"
    - "url"
    - "web"
    - "google"
    - "look up"
    - "find online"
    - "open page"
  patterns:
    - "(?i)(search|look up|find|browse)\\s.*(web|online|internet|google)"
    - "(?i)(go to|open|visit|navigate to)\\s.*(https?://|www\\.)"
    - "(?i)what (is|are|does).*(website|page|url)"
  tags:
    - "web"
    - "browser"
    - "search"
  max_context_tokens: 256
---

# Web Browse

Use the `browser` MCP tool (Playwright) to navigate and extract content from websites.

## Actions
- `browser_navigate`: open a URL
- `browser_get_text`: extract visible text from page or element
- `browser_screenshot`: capture page state
- `browser_click`: click element by CSS selector
- `browser_type`: type into input field

## Rules
- Prefer HTTPS URLs; reject `file://` URLs
- Extract only the text needed — avoid full-page dumps
- For searches: navigate to a search engine, type query, extract results
- Max 3 page loads per task unless user requests more
- Summarize extracted content; don't return raw HTML
