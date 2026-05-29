# Security Policy

## Reporting a vulnerability

Please report security issues privately rather than opening a public issue.

Use GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
("Report a vulnerability" under the repository's **Security** tab).

Please include:

- A description of the issue and its impact.
- Steps to reproduce, or a proof of concept.
- Affected version or commit.

You can expect an initial response within a reasonable time. Once a fix is
available, it will be released and the report disclosed.

## Scope and design notes

- Authentication uses OAuth 2.0 + PKCE. There is **no client secret**; the
  Spotify client id is a public identifier supplied via the
  `SPOTIFY_ARCHIVIST_CLIENT_ID` environment variable.
- Access and refresh tokens are stored in the operating system keyring, not in
  plaintext on disk.
- All data (tracked playlists, songs, sync history) is stored locally in
  SQLite. The app makes no network calls other than to the Spotify API.
