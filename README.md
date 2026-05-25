# elevator
An elevator simulation with a collection of traffic patterns and algorithm implementations.


## Usage
```
cargo run --release
cargo run --release -- 20 4
cargo run --release -- <floors> <num elevators>
```



## Overview

The elevator `Policy` (in `src\policy.rs`) gets notified of requests and arrivals, assigns elevators to incoming passengers, and sets the directions of elevators. Feel free to implement your own and open a PR. I made a couple of (dumb) policies and asked Gemini and ChatGPT to come up with some too.

`Traffic` (in `src\traffic.rs`) determines source and destination floors, and when and where requests are sent.