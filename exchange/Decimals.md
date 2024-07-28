# Why you need f64

There are some "good" arguments everywhere suggesting you to choose Decimal instead of f64

Decimal are good for internal controlled finincial systems, like a retail management system, exchange, etc.

But for trading systems, it's not always the case

## Performance

Decimal cost 10x time on every operation

```bash
git clone https://github.com/paupino/rust-decimal/
cd rust-decimal
cargo criterion
```

```text
addition/f64 (diff)/100                         time: [548.87 ps 565.59 ps 583.58 ps]     
addition/f64 (equal)/100                        time: [537.25 ps 548.20 ps 559.82 ps]
addition/rust-decimal (diff)/100                time: [5.5445 ns 5.5545 ns 5.5650 ns]
addition/rust-decimal (equal)/100               time: [5.2867 ns 5.3015 ns 5.3162 ns]
division/f64 (diff)/100                         time: [557.15 ps 578.31 ps 600.74 ps]   
division/f64 (equal)/100                        time: [542.87 ps 560.00 ps 579.80 ps]
division/rust-decimal (diff)/100                time: [18.646 ns 18.695 ns 18.752 ns]
division/rust-decimal (equal)/100               time: [6.5995 ns 6.6125 ns 6.6274 ns]
multiplication/f64 (diff)/100                   time: [520.95 ps 533.24 ps 547.83 ps]
multiplication/f64 (equal)/100                  time: [543.01 ps 561.07 ps 582.32 ps]
multiplication/rust-decimal (diff)/100          time: [5.3946 ns 5.4098 ns 5.4246 ns]
multiplication/rust-decimal (equal)/100         time: [5.3463 ns 5.3709 ns 5.4015 ns]
subtraction/f64 (diff)/100                      time: [534.58 ps 546.43 ps 559.12 ps]
subtraction/f64 (equal)/100                     time: [578.52 ps 598.97 ps 620.89 ps]
subtraction/rust-decimal (diff)/100             time: [5.5233 ns 5.5345 ns 5.5480 ns]
subtraction/rust-decimal (equal)/100            time: [5.3138 ns 5.3290 ns 5.3444 ns]

```

## Rounding and truncation error

f64 has rounding and truncation errors, Decimal can specify the decimal point it rounds

Note: f64 has 15 decimal points that it can precisely represent

Say price of 0.1 doesn't have an exact form in binary format, but when displaying and generating network requests,
actual rounding according to the exchange's requirement will be applied here so it become exact "0.1".

In a trading system, as price is always change, a small rounding doesn't matter.

It makes really little difference to calculate

Note: the example is based on f32 for brevity

```text
"3.14" * "0.1" = 0.314
3.14 * 0.1 = 3.1400001049041748046875 * 0.100000001490116119384765625 = 0.31400002
%error = %0.000006
```

Special handling is needed in orderbook though, as sometimes we need to use f64 as the key

However, as soon as the same constant str -> f64 conversion algorithm is used for the same market,

3.14 is always 3.1400001049041748046875 and be fit into the same slot in a map

We don't do operations like "what is the size of price 3.14 in orderbook look like"

## Cryptocurrencies

First claim, it's good enough for most common cryptocurrencies, BTC, ETH, SOL etc

Prices are from 0.0001 to 100000.00. It falls nicely into f64's precision.

Let's take some extreme examples.

Curreny price of SHIB is 0.00002647, and one need to buy at least 1 SHIB on BinanceSpot, it's within the precision

If one buys (1Billion-1)=99999999 SHIB, it's $26470 USD, decimal points are 4 + 9 = 13, still a breeze to handle it with
f64

PEOPLE/BTC price=0.00000041, we assume size is still 99999999999, it's 40999.99999959 BTC !  it's 2+11=13 decimal points

Some other exchanges may enforce a restriction on the step size, say 1000SHIB, and it will be easier to handle too



> Most crypto types are 128-256 and sometimes 512 bit sizes

1. Listed coins are don't need the high bits. $\log2(10^{15})=49.82892142$ I haven't seen a listed coin that it needs to
   operates on 50 bits, not even SHIB and PEOPLE in exchanges. Even if there are something to trade, it makes no real
   differences to trade 1000000000000000 vs 1000000000000314. it's less than a penny's difference not to mention
   USDT/USDC has only 6 decimal
   points. [A list of stable coins](https://gist.github.com/chiro-hiro/81fa40e69bc98773b57701ad106e96f6)
2. It only makes tiny sense is that you want to transfer or withdraw exactly 1000000000000314.15926535 coins, but again,
   why bother with it

## Pains with Decimal

One pain point with decimal is that you need to take Decimal into consideration designing an algorithm.

But the decimal rules are different from each symbol and each exchange.

e.g. hyperliquid requires 5 **significant digits** on prices, and there has to be a tool to enforce it in the end

But even if you enforce 5 significant, you have to do it in the end, when sending orders, not in the middle

1. you lose precisions if you do arithmetics on prices
2. Decimal doesn't support significant digits, only fixed decimal points
3. many operations may change Decimal's decimal, and you have to set decimal points again in the end
4. even if you have set decimal points, if it's wrong, the whole order will be rejected or the exchange impl have to
   check again

Another point is with middle price of orderbook. Usually for bid=100 ask=101 you have numbers like 100.5, and with
Decimal(decimal point 0), you can't represent it.

## Alternatives

We can add additonal fields instead. if an symbol requires really high precision, it can refer to price_hp if it's set
explicitly

```rust
struct Order {
    price: f64,
    price_hp: Option<Decimal>,
    size: f64,
    size_hp: Option<Decimal>
}
```

