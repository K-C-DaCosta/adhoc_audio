# Adhoc Audio
Audio compression written in pure rust. 
**It Doesn't** link to any bindings so its buildable for wasm32-unknown-unknown. 

## What is this? 
Its an audio codec I created on the fly for compressing audio.

## Why?
During the development of WASM application, the need arose to compress microphone data coming from the WEBAUDIO api. To keep building the project simple I needed the  encoder to be written in *pure rust*. AFAIK, there are a few pure rust audio **decoders** for things like VORBIS(lewton) ,MP3(puremp3) etc but most of those crates do not support **encoding**. 
