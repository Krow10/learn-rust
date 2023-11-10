# Slot machine

## Current game state

![wip_ui_demo_7_optimized](https://github.com/Krow10/learn-rust/assets/23462475/efb15513-7890-4ba6-8ab6-b28e26899595)

### Features

- Choose a game to play:
	+ Generic → a simple 2 coin game based on a 1987 real-life industry [par table](https://easy.vegas/games/slots/par_sheets/generic-1987.gif).
	+ Blazing7 → an implementation of the popular 3 coin game ([par table](https://easy.vegas/games/slots/par_sheets/Blazing7s.pdf)).
- Increment your bet with <kbd>j</kbd>, <kbd>+</kbd>, <kbd>↑</kbd> and decrement with <kbd>k</kbd>, <kbd>-</kbd>, <kbd>↓</kbd>.
- Spin the reels with <kbd>SPACE</kbd> or <kbd>ENTER</kbd> ! You can also skip the animations by pressing those keys again if you want.
- Watch your balance go up and down as you play. If it reaches zero, it's over !
- Press <kbd>q</kbd> or <kbd>ESC</kbd> to go back to the previous screen. <kbd>CTRL</kbd> + <kbd>C</kbd> will exit the app from anywhere.

## How to play

> **Note**
>
> This requires `Cargo` to be installed. See instructions on the [Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html) website for installation.

1. Clone the repo
```
$ git clone git@github.com:Krow10/learn-rust.git
```
2. Cd to `slot-machine` directory
```
$ cd learn-rust/slot-machine
```
3. Run the daemon
```
$ cargo run --bin daemon
```
4. In another terminal, `cd` again to `learn-rust/slot-machine` and run the client
```
$ cargo run --bin client
```
5. Start playing !

## Research

> **Note**
>
> This document holds the research done in order to implement the slot machine simulation. It is presented in note-taking format and is subjected to frequent changes. Some portions might be out-of-date.

- Payout ratio
	+ From 85% (low) to 98% (high) [2]
	+ Depends on machine *denomination* (example from [5])

| Denomination | Payback % |
|--------------|-----------|
| 1¢           | 90.09     |
| 5¢           | 92.47     |
| 25¢          | 91.48     |
| $1.00        | 92.81     |
| $5.00        | 93.31     |
| $10.00       | 94.47     |
| $25.00       | 95.92     |
| $100.00      | 98.85     |
| Average      | 91.88     |

- Reels and stops
	+ Number of symbols on each reel will affect the probability of near-miss for example (within regulation) [3]
	+ Virtual reels can increase **massively** the payout amount while still having the odds in favor of the house
		* Maps each "physical" stops to thousands or possibly infinite number of stops on the wheel *while* still keeping the
		  appearance of having only 3–5 reels
		* The display reel is completely separate from the virtual reel that is spun: the number of symbol and their order on the display is purely
		  for visual reference and aesthetics. It only requires for all the symbols from the virtual reel to be present (see [7] for example) as they are
		  weighted differently than how they appear in display (although from an implementation POV you would assign each physical stops to a certain range
		  even though they might refer to the same symbol). 
- Paylines
	+ The more, the better (~100)
	  [4]
	  > One of the tricks that casino game designers learned, was that if you were winning on one payline you didn't pay attention to the fact that you were losing on two of the other ones.
- Design [4]
	+ Make it look like a physical reel → Players trust them
		* Curved display
	+ Have an antagonist (can represent the Casino) or a goal to motivate player
		* Tell a story

## Implementation design

- Start with an easy version (*Easy Vegas* par sheet from [6])
- A tool for creating / managing par sheet (or simplified version) of the slot
	+ Have fun with Jackpot odds, RTP, hit frequency, etc.
	+ This means the slot machine (and animation / design and so forth) must be modular to accommodate for different number / shape of symbols,
	  number of stops, number of reels, etc.
- Spin machine controls
	+ Simple play (manual spin)
	+ Autoplay
		* Bet amount / paylines
		* Stop after XXX
		* See *Atkins Diet* example on [9] for inspiration
	+ Speed factor
	+ Allow showing live stats
		* Expected RTP, current RTP, EV, etc.
		
**What's been learned so far**
- ~~Physical reels~~ (don't care, just use virtual ones)
- Virtual reels dictates what you're going to show
	+ Abstract the symbols even more using indices from a symbols array
	+ If possible, try not to store the entire reels in memory (work with ranges ?)
- Describe combos as bit masks

### UI

- Single payline with **curved** display option (allows seeing the symbols above and below the middle payline)
- Give a window (Height x Width) that display the symbols (e.g. 3x5 for a five reels machine with multiple paylines)

### Architecture

Client / server could make it so that the frontend display be independent of all spin calculation / validation. That would also make it more modular and extendable to add new machines: add the par sheet to the back-end and create new UI front-end, everything else (e.g. RNG, validation, communication) is already there to be handled.

#### Server

- Par sheet calculation (offline)
	+ Store all relevant information into appropriate structs
- RNG (offline)
- Player data handling
	+ Balance
	+ (Bonus) Stored game state
	+ (Bonus) Identity validation / authentication
- Communication service / protocol (online)
	+ Receive client info and parse / validate
		* Machine to play
		* Bet amount vs. current balance
		* Pay lines
	+ Return spin result
		* Total winnings (calculate from paylines, bet amount, etc.)
		* Payload: stops, winning lines, total winnings

#### Client

- UI front-end (offline)
	+ Display symbols and animation
	+ Handle all paylines' setup, bonus rounds, etc.
	+ User input / autoplay
- Communication service (online)
	+ Connect to server and send command to spin
	+ Receive payload
	
#### Communication protocol

Using UNIX sockets (`/tmp/slot_machine.sock`).

**Client → Server**
`PLAY {GAME} {BET}` select a game to play with the payouts sized with the bet.

**Server → Client**
`SPIN {WIN} {BALANCE} {REEL 1} {REEL 2} ... {REEL N}` returns the current balance and win amount based on the rolled combination.
`ERRX {MSG}` returns an error message (e.g. insufficient balance).

### How to map virtual reel to physical / display reel

1. Assign each symbol a probability of hit `p[i]` with `sum(p[i]) = 1`
2. Pick a total number of virtual stops `n` and compute `k = p[i] * n` for each symbol
3. Find out the count `c` of each symbol on the physical / display reel and compute `r = k//c`
4. For each symbol on the physical / display reel, assign a continuously increasing range of `r` numbers or the remainder `k - r*(c-1)` if `k % c != 0`

Each reel can thus be weighted differently. This is mostly used to create the *near-miss* effect. You could do this in two different ways :
1. In step 4, don't assign the ranges uniformly but increase the range of blanks (or other symbol) near the jackpot symbol so that they're more likely to appear.
2. Decrease the count of the jackpot symbol as you approach the last reels.

#### Example of a mapped physical / display reel to virtual stops

| Stop | Symbol | Range | Number of Chances |
|------|--------|-------|-------------------|
|1|cherry|1-2|2|
|2|[BLANK]|3-7|5|
|3|—|8-12|5|
|4|[BLANK]|13-17|5|
|5|7|18-25|8|
|6|[BLANK]|26-30|5|
|7|—|31-35|5|
|8|[BLANK]|36-41|6|
|9|cherry|42-43|2|
|10|[BLANK]|44-49|6|
|11|==|50-56|7|
|12|[BLANK]|57-62|6|
|13|cherry|63|1|
|14|[BLANK]|64-69|6|
|15|==|70-75|6|
|16|[BLANK]|76-81|6|
|17|—|82-87|6|
|18|[BLANK]|88-93|6|
|19|ΞΞ|94-104|11|
|20|[BLANK]|105-115|11|
|21|jackpot|116-117|2|
|22|[BLANK]|118-128|11|

> **Note**
>
> The sum of *Number of Chances* for each symbol amounts to `k` and that each number represents either `r` or `k - r*(c-1)`. See [7] for more explanations.

## References

1. https://www.youtube.com/watch?v=jQIHqkudgNY - Gambling and the desire machine | Pay to Win
2. https://www.youtube.com/watch?v=7Wkubf1PrWg - Slot Machines - How to Win and How They Work • The Jackpot Gents
3. https://www.youtube.com/watch?v=LvgsGfbgItQ - IP11: Greg Dunlap — Are slot machines rigged?
4. https://www.youtube.com/watch?v=1B5UHZhimVQ - State-of-the-Art Slot Machine Design | Al THOMAS
5. https://www.americancasinoguidebook.com/slot-machines/slot-machine-payback-percentages-can-they-help-you-win.html
6. https://easy.vegas/games/slots/par-sheets
7. https://easy.vegas/games/slots/how-they-work
8. https://easy.vegas/games/slots/program
9. https://wizardofodds.com/games/slots/atkins-diet/
10. https://easy.vegas/games/slots/par_sheets/generic-1987.gif
