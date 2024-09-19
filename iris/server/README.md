

# Iris Project - Rust re-implementation

## Project Structure
``````

``````
## Streaming Server

- [ ] *Source Stream Handler* - handles incoming stream encoded in MPEG-TS
- [ ] *Network Scheduler Framework* - pipeline of packets, retransmissions, filtering, predictions and more
- [ ] *Security/Membership Manager* - handles clients certificates signing and more
- [ ] *Crypto/Key Store* - stores credentials in sqlite database

### Source Stream Handler

MPEG-TS Stream Source, can be:
- File Source - reads from a .ts file
- FFMPEG Source - reads from a encoder ffmpeg

### Crypto/Key Store

Uses gRPC calls named SignCertificate and VerifyCertificate

- SignCertificate : takes a CertificateRequest that contains a CSR Request, sines it and thens the certificate back as Response
- VerifyCertificate : takes a certificate and verifies that is signed by the CA

This all runs in the gRPC Server that currently runs as blocking in the main thread (needs to be spawn in another thread)


## Edge Cloud

### Network Scheduler

This is a Pipeline that includes multiple schedulers

- Wifi Multicast Scheduler : this is the primary and fallback is any other fails along the pipeline
- Wifi Unicast Scheduler 
- 4G Unicast Scheduler (not gonna implement)

This pipeline also as a Prediction Service using Machine Learning and a Client Sessions 