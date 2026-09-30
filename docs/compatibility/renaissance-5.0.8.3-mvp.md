# Renaissance 5.0.8.3 MVP compatibility contract

This contract describes the narrow ServUO reference path for a classic client. It
does not assert that an untested client accepts RustUO's proposed direct spawn.

| Stage | Client input | ServUO response and state |
| --- | --- | --- |
| Login seed | Four nonzero, big-endian bytes | `MessagePump.HandleSeed` consumes the prefix before packet framing. |
| Account login | `0x80`, 62 bytes: two 30-byte credential fields and a trailing byte | `AccountLogin` accepts only a first packet. `AccountHandler` checks the XML-backed account, access, password and ban state. Success sends `0xA8` server list; rejection sends `0x82` and closes. |
| Server selection | `0xA0`, 3 bytes: signed, big-endian server index | `PlayServer` requires an offered index and account, issues a reconnect auth ID, and sends `0x8C` with IPv4 bytes, big-endian port and auth ID. |
| Game reconnect | New connection: seed, then first-packet `0x91`, 65 bytes: auth ID and two 30-byte credential fields | `GameLogin` consumes the one-use auth ID, verifies the account again, enables outbound compression, sends supported features and, for this pre-7.0.13 client, `0xA9` old character list. |
| World entry | `0x5D`, 73-byte character-slot request | `PlayCharacter` binds the account-owned mobile. `DoLogin` sends `0x1B` login confirm, map/feature/mobile state, then `0x55` login complete and further state. |
| Movement | `0x02`, 7 bytes: direction, sequence, four-byte key | `MovementReq` delegates to `Mobile.Move`. A successful move/turn sends `0x22` sequence/notoriety acknowledgement; rejection sends `0x21` sequence/location correction and resets the sequence. |

## Reference evidence

- `legacy/Server/Network/MessagePump.cs:139-174,206-233` — old seed and packet framing.
- `legacy/Server/Network/PacketHandlers.cs:66-108,1780-1805,2404-2585,2961-3059,3119-3171` — handler registration, account/server/game flow, character play, world entry and movement.
- `legacy/Server/Network/Packets.cs:4498-4579,4842-4903,4963-4969,5092-5113,5150-5164` — outbound packet identifiers and layouts.
- `legacy/Server/Network/NetState.cs:215-225,261` — old versus new character-list version boundary.
- `legacy/Server/Mobile.cs:3103-3140,3339-3349` — movement/turn and acknowledgement.
- `legacy/Scripts/Accounting/Accounts.cs:54-78`, `legacy/Scripts/Accounting/AccountHandler.cs:251-332,334-409`, `legacy/Scripts/Misc/ServerList.cs:29-47` — XML load, credential checks and offered endpoint.

## MVP boundary and unresolved proof

RustUO may authenticate from legacy XML and use one pre-seeded in-memory avatar,
without implementing normal character selection. ServUO source **does not prove**
that an actual 5.0.8.3 client will accept world-entry packets without first
receiving `0xA9` and sending `0x5D`. A real-client smoke test must resolve this.
RustUO's compatibility and loopback tests exercise seed handling, packet layouts,
TCP login/reconnect composition, compression, world entry, and a movement
request/response using synthetic wire fixtures. They do not establish that an
actual 5.0.8.3 client accepts the pre-seeded spawn or completes the exchange;
that real-client acceptance remains the R5-MVP-04 gate.
