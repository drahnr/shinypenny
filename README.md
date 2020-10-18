# shinypenny 🪙

A small helper tool to collect and concatenate receipts data with tax and value annotations from a csv or command-line.

Create a fully sanitized reimbursement request with

`shinypenny --csv ./monopoly.csv`

or

`shinypenny --csv ./monopoly.csv reimbursement_request.pdf`

or for a single data entry, you can pass all items via command-line
flags see `shinypenny --help`.

## License

There are certain artifacts included, i.e. fonts and pivot image.

* Roboto Fonts as taken from `google-roboto-fonts-2.138-6.fc32.noarch` and is pubished under [`Apache-2.0`](https://fonts.google.com/specimen/Roboto#license).
* Test image by [Jonathan Brinkhorst](https://unsplash.com/@jbrinkhorst) under the [The Unsplash License](https://unsplash.com/license).
* Source code is under `Apache-2.0 OR MIT`.

## Configuration

Configure the destination bank account by setting these two vars
accordingly in your `shinypenny.toml` configuration file

```toml
name = "Roger Ronjason"
iban = "NO1876..........909"

[company]
name = "Big $ Corp"
address = "Strahlemax Str. 20, 1111 Irgendwo"
```

which resides in (given your username is `Alice`)

`/home/alice/.config/` Linux
`C:/Users/Alice/AppData/Roaming` Windows
`/Users/Alice/Library/Application Support` Mac OS

## CSV

The format is determined by by the header row, which can be omitted if the order
as kept in the example below. If the columns are re-ordered, the header tags must be provided
with the names as shown below.

By default `|` is used as separator, a secondary attempt is made with `;`, and tertiary with `,`.

Numbers and decimals may be delimited with `.` characters independent of the locale - `,` is not a valid decimal separator, see [the rust `f64` grammar](https://doc.rust-lang.org/std/primitive.f64.html#grammar).

Receipt paths are relative to the `cwd`.

```csv
date      |company     |description                    |netto |tax |brutto|path
2020-09-20|watercorp   |Device: Superblaster 2k21  |100.00|0.05| 95   |spensiv.pdf
2020-09-20|OfflineBooks|How to create a wormhole. |100   |0.05| 95.00|funny.pdf
2020-09-20|OfflineBooks|Yaks, to shave or not to | 10   |0.16|  9.40|001_receipt.pdf
2020-09-20|Prepers. Inc|Doomsday prep day |111   |0.16| 93.24|dpd.pdf
```

but also with `€` and `%` annotations.

```csv
date      |company     |description                    |netto |tax |brutto|path
2020-09-20|watercorp   |Device: Superblaster 2k21  |100 €|5 %| 95   |spensiv.tiff
2020-09-20|OfflineBooks|How to create a wormhole. |100 €|0.05| 95.00 €|funny.jpeg
2020-09-20|OfflineBooks|Yaks, to shave or not to | 10   |16 %|  9.40|001_receipt.pdf
2020-09-20|Prepers. Inc|Doomsday prep day |111   |0.16| 93.24|dpd.png
```

## Roadmap

This is a purely necessity driven project.

* [ ] Support entries other than euros (€)
* [ ] Allow specifying a pivot pdf page with a designate table area
* [ ] Replace `0.00` values with a `-` within the table

If you need a particular feature, please open an issue before filing a pull request.
