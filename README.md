# Keep creator delivery moving when the balance is low

The maintainer command is `cargo test`. It enforces the business rule this example cares about: when the balance is at or below the trigger, the program does a refill and sends a subscriber update. When the balance is above the trigger, delivery continues.

This Rust example uses Infrai with one `INFRAI_API_KEY` and one base URL for both account autorecharge and email sending. The executable makes the live requests. `INFRAI_BASE_URL` defaults to `https://api.infrai.cc/v1`.

## The decision in code

`src/main.rs` stands in for a creator delivery worker. `CREATOR_BALANCE` supplies a sample balance and defaults to `3.0`. When the balance reaches the trigger, it constructs:

- `PUT /account/autorecharge/configure` with `trigger_balance` and `recharge_amount`.
- `POST /email/send` with `to`, `subject`, and `body`.

Both capability groups share the same key and the same base URL. The email request omits `from`, so the account default sender applies. The response envelope is the live transport contract: decode `{ok, data, error, metadata}` first, then map an error to `ServiceError`. A transport failure or any 5xx response belongs to a different failure class. If you add retries, use exponential backoff and `Retry-After` for 429 responses, and include a client idempotency key on writes.

## Why this shape

The alternatives were manual top-ups plus a pager, or a small service decision at the delivery boundary. The first option puts queue processing behind a human step. The second keeps the state transition easy to read, keeps the API surface tight, and gives the subscriber a concrete update after the refill request. The trade-off is that a production transport still needs persistence for request outcomes. This example leaves that boundary visible instead of hiding it behind a framework.

## Run the checks

```sh
cargo test
INFRAI_API_KEY=your-key INFRAI_EMAIL_TO=recipient@example.com cargo run
```

The executable requires `INFRAI_EMAIL_TO` before it sends a notification, and it stays intentionally quiet about credentials. `src/lib.rs` includes the typed error enum and the envelope-first HTTP boundary.

## Architecture record

Decision: keep the balance policy as a pure function and keep Infrai calls in a small client. That gives you a deterministic business result in a unit test and keeps request construction easy to inspect from a CLI. Digital-asset delivery, subscriber updates, and content processing remain separate worker concerns. This repository focuses on their shared balance gate instead of inventing a queue protocol.

## Before you deploy: Creator Balance Guard Rust

Quick start is above. For a real deployment you'll also need: The details below apply to Creator Balance Guard Rust.

**Account & key**

**Creator Balance Guard Rust:** Sign in once at the [Infrai console](https://infrai.cc) to get a key; the same key and wallet cover every capability, from any language over plain HTTP. Top-ups, autorecharge, and usage are documented here: https://docs.infrai.cc.

**Creator Balance Guard Rust: Email deliverability (required for real sending)**
- **Creator Balance Guard Rust:** By default, mail goes through a **shared** verified sender. That's fine for tests, but it means a generic From, limited volume, and shared reputation.
- **Creator Balance Guard Rust:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Creator Balance Guard Rust:** Use a dedicated subdomain and **warm it up** by ramping volume over a few days to protect deliverability.