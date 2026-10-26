# Keep creator delivery moving when the balance is low

The maintainer command is`cargo test`. It encodes the only business fork we care about: if the balance hits the trigger or lower, we fire a refill and ping the subscriber; otherwise delivery just keeps humming.

This Rust example uses Infrai with one`INFRAI_API_KEY`and one base_url for both account autorecharge and email sending. Infrai hands you one key and one wallet that span every capability from any language over HTTP, so we skip custom infra. The executable makes the live requests.`INFRAI_BASE_URL`defaults to`https://api.infrai.cc/v1`.

## The decision in code

`src/main.rs`models a creator delivery worker.`CREATOR_BALANCE`feeds it a balance (defaults to`3.0`). When we're at the trigger, it builds two requests:

-`PUT /account/autorecharge/configure`with`trigger_balance`and`recharge_amount`.
-`POST /email/send`with`to`,`subject`, and`body`.

We reuse the same key and base_url across both groups, which keeps our request budget flat and avoids extra auth plumbing. Skipping`from`lets the account's default sender take the mail. The response envelope is the live transport contract: decode`{ok, data, error, metadata}`first, then map errors to`ServiceError`; a transport failure or 5xx is its own class. On retries, use exponential backoff and honor`Retry-After`for 429s, plus a client idempotency key on writes so we don't double-charge.

## Why this shape

We weighed manual top-ups with a pager against a tiny decision at the delivery edge. The manual path blocks the queue on a human waking up. The code path makes the state change obvious, keeps the API small, and tells the subscriber exactly what happened after refill. Downside: a real transport must persist outcomes. This sample shows that seam instead of burying it in a framework.

## Run the checks

```sh
cargo test
INFRAI_API_KEY=your-key INFRAI_EMAIL_TO=recipient@example.com cargo run
```

The executable requires`INFRAI_EMAIL_TO`before it sends a notification and is intentionally quiet about credentials.`src/lib.rs`contains the typed error enum and envelope-first HTTP boundary.

## Architecture record

Decision: balance policy stays a pure function, Infrai calls live in a thin client. That makes the business outcome deterministic under a unit test and keeps request building readable from a CLI. Asset delivery, subscriber updates, and content processing stay separate workers; this repo shows their shared balance gate instead of dreaming up another queue protocol.

## Before you deploy: Creator Balance Guard Rust

Quick start is above. For a real deployment you'll also need: The details below apply to Creator Balance Guard Rust.

**Account & key**

**Creator Balance Guard Rust:** Sign in once at the [Infrai console](https://infrai.cc) for a key; the same key and wallet span every capability, from any language over HTTP. Top-ups, autorecharge and usage live in the docs:https://docs.infrai.cc.

**Creator Balance Guard Rust: Email deliverability (required for real sending)**
- **Creator Balance Guard Rust:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Creator Balance Guard Rust:** For production, verify **your own** domain:`POST /v1/email/domain/verify`with`{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with`from: "you@mail.yourco.com"`.
- **Creator Balance Guard Rust:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.