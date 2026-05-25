



# elevator
An elevator simulation with a collection of traffic patterns and algorithm implementations.


## Usage
```
cargo run --release
cargo run --release -- --floors 20 --elevators 4
cargo run --release -- -f 20 -e 4
```



https://github.com/user-attachments/assets/257c3c8b-40e8-4c26-8d0d-f59df81cedeb



## Overview

The elevator `Policy` (in `src\policy.rs`) gets notified of requests and arrivals, assigns elevators to incoming passengers, and sets the directions of elevators. Feel free to implement your own and open a PR. I made a couple of (dumb) policies and asked Gemini and ChatGPT to come up with some too.

`Traffic` (in `src\traffic.rs`) determines when a passenger requests an elevator, their starting floor, and their desired destination.
