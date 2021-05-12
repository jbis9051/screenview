# The Weak Pre Shared Key, Key Authentication (WPSKKA) Protocol

## Introduction

In recent years end-to-end encryption has risen in popularity due to privacy and
security concerns. However, many implementations of end-to-end encryption rely
on a third party and/or are susceptible to man-in-the-middle attacks making them
inadequate.

Screen sharing applications are used for a multitude of different purposes. One
common use case is an IT professional assisting somebody by remotely viewing and
controlling their computer. Sensitive data could be visible on the user's
screen. Therefore, end-to-end encryption is preferred.

Additionally, IT professionals will often be communicating with the user via the
telephone. This provides a bi-directional external channel to transfer
information. However, people cannot and will not transfer large amounts or
complicated data reliably. Simply communicating letters can be confusing. "B",
"C", "D", "E", "G" all sound similar and can be confused. Therefore,
communicating strictly numbers is ideal.

This creates an issue. Short, purely numeric keys provide extremely low entropy.
A 10 digit numerical code only provides about 33-bits of entropy. In fact, in
order to get the ideal 128-bits of entropy a 39 digit key would need to be used.
Users will not want to relay 39 digits over the phone.

One common solution when using a weak password is using a KDF to perform
key-stretching. However, a 10 digit numerical code has such a small key space
that it is relatively easy to brute force. Government agencies, such as the NSA,
or large companies, such as Microsoft or Google could easily brute-force even
very slow KDFs such as argon2 or PBKDF2.

### Existing Practice

Based on
[TeamViewer's security statement](https://static.teamviewer.com/resources/2017/07/TeamViewer-Security-Statement-en.pdf),
TeamViewer is end-to-end encrypted. However, a rogue or malicious TeamViewer
intermediary server could easily provide a different set of keys to each party
and intercept all communication. The parties must trust this third party in
order to achieve proper end-to-end encryption. TeamViewer also does not give the
ability for users to check the fingerprint of the other party's public key.

Other applications such as Zoom and Signal (and the Signal protocol itself) do
give users the ability to verify some sort of fingerprint of the public key
([Zoom](https://support.zoom.us/hc/en-us/articles/360048660871-End-to-end-E2EE-encryption-for-meetings#h_01ENGDKFFBKTF796CE03FTCH6J)
and [Signal](https://signal.org/blog/safety-number-updates/)). However, most
users don't bother confirming the numbers. Again trusting the third party
server.

Solutions such as TLS do provide strong end-to-end encryption but rely on a
third party Certificate Authority to sign public keys. This would not be
possible in the aforementioned use case.

### Goal

Elliptic Curve Diffie-Hellman provides a sufficiently secure means of arriving
at a shared secret. The goal of this protocol is to authenticate public keys
using a weak (3 bytes length, 24 bits of entropy) pre-shared key communicated
via an external channel. The security of the external channel is out of scope
for this protocol and will be assumed to be secure (see Security Considerations
section).

### Requirements

Communication between the parties should be minimized.

The protocol should be secure in the case of a malicious Client and/or malicious
Server. This means the protocol should not be susceptible to man-in-the-middle
(MITM) attacks, and the Host should be able to self authenticate the Client
without trusting the Server.

### Definitions

The following definitions are used:

1. Host - The user that wants to share their screen to the Client
2. Client - The user that wants to view and control the Host's screen
3. Server - The intermediary server used for routing and proxying data between
   the Host and the Client

### Other Protocol/Algorithms Definitions

The Secure Remote Password (SRP) protocol is defined in
[RFC2945](https://datatracker.ietf.org/doc/html/rfc2945) and
[RFC5054](https://datatracker.ietf.org/doc/html/rfc5054).

Elliptic Curve Diffie-Hellman (ECDH) key exchange is described in
[RFC6090](https://datatracker.ietf.org/doc/html/rfc6090).

## Protocol

This protocol occurs after a connection is established between the Host and
Client using the Server.

The SRP group is the 2048-bit group from RFC5054:

The hexadecimal value for the prime is:

```
AC6BDB41 324A9A9B F166DE5E 1389582F AF72B665 1987EE07 FC319294
3DB56050 A37329CB B4A099ED 8193E075 7767A13D D52312AB 4B03310D
CD7F48A9 DA04FD50 E8083969 EDB767B0 CF609517 9A163AB3 661A05FB
D5FAAAE8 2918A996 2F0B93B8 55F97993 EC975EEA A80D740A DBF4FF74
7359D041 D5C33EA7 1D281E44 6B14773B CA97B43A 23FB8016 76BD207A
436C6481 F1D2B907 8717461A 5B9D32E6 88F87748 544523B5 24B0D57D
5EA77A27 75D2ECFA 032CFBDB F52FB378 61602790 04E57AE6 AF874E73
03CE5329 9CCC041C 7BC308D8 2A5698F3 A8D0C382 71AE35F8 E9DBFBB6
94B5C803 D89F7AE4 35DE236D 525F5475 9B65E372 FCD68EF2 0FA7111F
9E4AFF73
```

The generator is: 2.

The Host generates the following:

- _PK<sub>H</sub>/pk<sub>H</sub>_ - Host ephemeral elliptic curve Public/Private
  key using the `secp521r1` (P-521) curve
- _I_ - 128 bit cryptographical secure random number, used as the identity or
  username in SRP
- _S_ - SRP salt
- _P_ - 3 byte random cryptographical secure random number, used as the password
  in SRP
- _V_ - SRP verifier
- _b_ - SRP random private value
- _B_ - SRP public value
- _k_ - SRP K value

The Host sends S, I, and B to the Client.

The Client generates:

- _PK<sub>C</sub>/pk<sub>C</sub>_ - Client ephemeral elliptic curve
  Public/Private key using the `secp521r1` (P-521) curve
- _a_ - SRP random private value
- _A_ - SRP public value
- _u_ - SRP u value
- _k_ - SRP k value
- _x_ - SRP x value (hashed P value communicated externally to the client)
- _L_ - SRP session key

The Client sends the Host A, PK<sub>C</sub> and HMAC(PK<sub>C</sub>, L).

The Host derives:

- _u_ - SRP U value
- _L_ - SRP session key

The Host authenticates PK<sub>C</sub> HMAC using L. If authentication is
successful, the Host performs DHKE to derive, T the shared secret.

The Host sends the client PK<sub>H</sub> and HMAC(PK<sub>H</sub>, L).

The Client authenticates PK<sub>H</sub> HMAC using L. If authentication is
successful, the Client performs DHKE to derive, T the shared secret.

## Security Considerations

All authentication security is provided by the secrecy of _P_ during the initial
exchange. If the external channel used to communicate _P_ is actively
intercepted and an intermediary server is malicious, a MITM attack can be
conducted by an adversary. However, this must be an active attack as disclosure
of _P_ after the true _B_ value is known by the client renders this MITM attack
impossible.

A malicious server or client could attempt to brute force _P_. However, every
attempt requires interaction with the Host. After a few failed attempts, the
Host should generate a new _P_ value. If failed attempts continue, the Host
should stop accepting connections all together and report an issue to the user.
To prevent DOS (denial of service) attacks and/or a malicious client forcing the
Host to regenerate _P_, the Server should add rate limiting and other DOS
protection for Clients.

In order to ensure perfect forward secrecy, new key pairs
(PK<sub>H/C</sub>/pk<sub>H/C</sub>) should be generated for each session.
