# Icepeek

## Update Dec'2025

The concept is somwhat weakened by the time taken to download all the filter.
Nonetheless, Kyoto is improving. Update to [v0.15](https://github.com/2140-dev/kyoto/releases/tag/v0.15.0) is ongoing
(https://github.com/optout21/icepeek/tree/kyoto-15).


## Description
Watch-only bitcoin onchain balance checker, private and secure.
Can be used for checking cold wallet balances ("peek under the ice").

WARNING: Don't ever enter any seedphrase or private key in this project!

Warning: This software is prototype with no warranty whatsoever.

Based on compact block filters (BIP-147/148), peer-to-peer, not using servers.
Written in Rust, using [kyoto-cbf lib](https://github.com/2140-dev/kyoto).

Friendly Advice: Don't forget to regularly check your keys!


### Variants

- `icepeek-iced`: desktop version with UI, based on `iced` library (Rust)

- `icepeek-cli`: simple command-line based version (mostly for testing)


### History

The first version of Icepeek was based on
[Nakamoto CBF library](https://github.com/cloudhead/nakamoto),
but later (Oct2024) it was switched to the more recent
[kyoto-cbf](https://github.com/rustaceanrob/kyoto).


## How to run

### The (simple) command-line variant:

```
cargo r -p icepeek-cli
```

Wait and watch the output!


### The desktop UI variant:

```
cargo r -p icepeek-iced
```

- Optionally change the xpub & parameters

- Press Continue

- Wait and watch the info


## TODO

- show state (state Connecting, FastSync, ChilledSync; show number of peers)
- more user-friendly starting block supoprt (no hash needed, custom height, start filters from there)
- initial DNS-based peer discovery doesn't work with VPN (kyoto?)
- Address discovery (gap limit, add new addresses)
- download filters in reverse order


## Kyoto Qs

- report more progress events (peer connect/disconnect, etc.)


## Nakamoto Qs (obsolete)

- latest version is not released (can only be used with local build)
- filters are always retrieved again, even if they are cached (first they are loaded, then retrieved from the beginning)
- load never returns, scan cannot be started only after load
- separation of load and retrieve (wallet-independent), and scan (wallet dependent)

