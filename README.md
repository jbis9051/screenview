# ScreenView

A zero config remote screenshare and control. Open source alternative to TeamViewer 

## Abstract

ScreenView is a suite of cryptographical and application level networking protocols culminating in a
zero configuration end to end encrypted remote screen viewing and controlling software. ScreenView aims to
replace TeamViewer, RDP, and VNC for many use cases while being more performant and more secure. ScreenView
requires little set up and is just as easy or easier to set up than other solutions. ScreenView defines four different
layers of protocols, each encapsulating all the layers below it. Cryptography for communication between peers
and the server is based upon TLS 1.3 and Wireguard. End-to-end cryptography used for ALL communication between
peers is based upon TLS-SRP. ScreenView end-to-end cryptography prevents man-in-the-middle attacks even if
the intermediary server is compromised, unlike TeamViewer. Screen data is sent over UDP to achieve superior
performance than TCP based solutions such as VNC. All UDP packets must be authenticated with keys established
over TCP before a response is made by the server preventing amplification attacks. A congestion control
mechanism is used to handle low bandwidth and poor networking conditions. Finally, ScreenView supports
advanced use cases including file transfer, multiple displays, sharing specific windows, shared whiteboards,
and clipboard transfer[...](#protocol-documentation)


# Protocol Documentation

Protocol documentation can be found [here](docs/out/protocol.pdf).