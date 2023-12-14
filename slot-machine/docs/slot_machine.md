`slot_machine` is a [TUI](https://en.wikipedia.org/wiki/Text-based_user_interface) implementation of traditional slot machine games.

It runs as a client / server architecture where the server manages the randomness, win and balance calculation while the client is displaying the game to the user.

The crate is therefore split into two binaries, `client` and `daemon` (with a third one for simulating a slot machine's outcomes, not as developed as the other two). To run games, run the `daemon` with `cargo run --bin daemon` in one terminal window and the client (`cargo run --bin client`) in another one.

The client and server will exchange messages on the socket designated by the `SOCKET_PATH` variable. They will both try to parse games information from the `GAMES_FOLDER` directory.

**Example architecture**
```console
data/          
├── display_symbols.json
├── games                                                                                    
│   ├── blaze7
│   │   ├── display.csv  
│   │   ├── paytable.csv
│   │   ├── reels.csv
│   │   └── symbols.csv
│   └── generic                                                                              
│       ├── display.csv
│       ├── paytable.csv                                                                                                                                         
│       ├── reels.csv                                                                                                                                            
│       └── symbols.csv
└── symbols
    ├── banana.png
    ├── bar2.png
    ├── bar3.png
    ├── bar.png                                                                                                                                                  
    ├── bell.png                                                                                                                                                 
    ├── blank2.png
    ├── blank.png
    ├── blazing_seven.png
    ├── cherry.png
    ├── lemon.png
    ├── seven.png                                           
    ├── SOURCES.md
    └── watermelon.png
```
*Taken from the [project's repository](https://github.com/Krow10/learn-rust/tree/main/slot-machine)*

Each game is expected to be its own subdirectory containing a set of CSV files that are used to describe the symbols, reels[^reels] and pay table[^paytable]. The following sections will go into more detail about each file.

[^reels]: Columns where the symbols are spinning.

[^paytable]: A table outlining all winning combinations and their win values.

### `display_symbols.json`

This file is at the root of the `GAMES_FOLDER` and is used by all games as a catalog of available symbols for display on the reels. It follows a specific structure:
```json
{
	"name": "<Unique identifier for the symbol>",
	"path": "<A path to an image file (.png and .jpg supported, other formats not tested) representing the symbol to display>",
	"luma_threshold": "<An integer value representing the cutting point for selecting the brightest pixels from the image (experiments might be needed to find the optimal value)>",
	"color": "<An hex string value for the solid color that will be applied to the display symbol>"
}
```

### `display.csv`

A mapping of *display* symbols' identifier to *display* symbol names referenced in the `display_symbols.json` file.

**Example**

| Symbol identifier | Symbol display name |
|-------------------|---------------------|
| BL | blank |
| R7 | classic_seven |

### `symbols.csv`

A mapping of symbols' identifier to their pay table reference. The table shall at least include all the symbols from the `display.csv` table. The reason for having an additional symbol table is to be able to represent *classes* of symbol as a unique identifier in the pay table's combos.

For example, consider the following game rule *pay 10 if symbol S appear on any of the reels*. To implement such a rule, you can define three combos (in the case of a three reels game) that look like this:
```
S X X
X S X
X X S
```
With `X` being a *special* symbol representing *all symbols other than S*. That relationship would be expressed in the `symbols.csv` table as follows:

| Symbol identifier | Symbol reference |
|-------------------|------------------|
| S | S |
| S2 | S2 |
| S3 | S3 |
| X | !S |

In this game's implementation, any *display* symbol that is not an *S* (e.g. *S2*, *S3*) will match the *X* in the rule. Hence, all display symbols can be recognized by having their identifier equal to their reference in this table (they should be placed at the top by convention).

The supported logical operations for creating symbol references are:
- `!`: NOT
- `|`: OR

See this [file](https://github.com/Krow10/learn-rust/raw/main/slot-machine/data/games/blaze7/symbols.csv) from the project's repo as an example for a game implementation.

### `paytable.csv`

A mapping of *combos* to their payout values. The file shall contain at least one payout column. Each additional payout column increases the required bet size by `1`.

*Combos* are made up of `N` space-separated symbol identifier from the `symbols.csv` file, with `N` being the number of reels of the game.

**Example**

| Combo | Payout 1 | Payout 2 | Payout 3 |
|-------|----------|----------|----------|
| R7 R7 R7 | 50 | 100 | 200 |
| R7 BL BL | 4 | 4 | 4 |
| BL R7 BL | 4 | 4 | 4 |
| BL BL R7 | 4 | 4 | 4 |
| BL BL BL | 2 | 2 | 2 |

### `reels.csv`

Columns describing the placement (or stop in game terminology) of the display symbols on each of the reels. These reels correspond to what the user will see spinning while waiting for the result. They shall only contain *display* symbols identifier (from the `display.csv`). Unequal number of symbols on the reels is not supported (i.e. each reel must have the same total number of symbols).

**Example**

| Reel 1 | Reel 2 | Reel 3 |
|--------|--------|--------|
| BL | BL | BL |
| BL | BL | BL |
| R7 | BL | BL |
| BL | R7 | BL |
| BL | BL | R7 |
| BL | BL | BL |
| BL | BL | BL |