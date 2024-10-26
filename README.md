# Icepeek

## Description
Watch-only bitcoin onchain balance checker, private and secure.
Use it to check your deep cold wallet balances ("peek under the ice").

WARNING: Don't ever enter any seedphrase or private key in this project!

Warning: This software is prototype with no warranty whatsoever.

Based on compact block filters, peer-to-peer, not using servers.
Written in Rust, using [kyoto-cbf lib](https://github.com/rustaceanrob/kyoto).

Friendly Advice: Don't forget to regularly check your keys!


### Variants

- `icepeek-iced`: desktop version with UI, based on `iced` library (Rust)

- `icepeek-cli`: simple command-line based version (mostly for testing)


### History

The first version of Icepeek was based on
[Nakamoto CBF library](https://github.com/cloudhead/nakamoto),
but later (Oct2024) it was switched to the more recent
[kyoto-cbf](https://github.com/rustaceanrob/kyoto).


## TODO

- show state (show block heights; state Connecting, FastSync, ChilledSync; show number of peers)
- more user-friendly staarting block supoprt (no hash needed, custom height, start filters from there)
- Address discovery (gap limit, add new addresses)
- download filters in reverse order


## Kyoto Qs

- report more progress events (peer connect/disconnect, block header height, etc.)


## Nakamoto Qs (obsolete)

- latest version is not released (can only be used with local build)
- filters are always retrieved again, even if they are cached (first they are loaded, then retrieved from the beginning)
- load never returns, scan cannot be started only after load
- separation of load and retrieve (wallet-independent), and scan (wallet dependent)

