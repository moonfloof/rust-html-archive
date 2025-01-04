# HTML Archive

A very small piece of software to collect all HTML and plain text files within a given folder structure and convert them into a static site.

This software is/must be:

* **Portable** (can be moved around servers and still work)
* **Durable** (intended to last for as many years as possible)
* **Very simple** (nothing fancy, no javascript - just get all the files, bang some header on top, and output into a single folder)

## Writing Posts

* If the filename starts with an ISO date (eg. `2000-01-01`), that will be used as the published date. If _not_, the last modified date will be used instead.
* If the filename starts with "DRAFT", the post will not be published.
* If the file has a `txt` or `md` extension, it will be displayed on the page as plain text with no formatting.
* If the file has a `html` extension, it will be used exactly as-is. Literally any HTML can be included here, including scripts, iframes. It's your content after all, so all responsibility is placed upon you.
* Any styles, images, audio, videos can be copied over to the same folder, assuming they are sourced locally in your HTML file. For example: `<img src="./image.jpg" />` will copy `image.jpg`). The detection is based on only the quotation marks and the dot slash.
* Because of a Windows limitation in what can be contained in filenames, the following phrases will be converted to their relevant symbol:
  * `{{q}}` -> `?`
  * `{{c}}` -> `:`
  * `{{s}}` -> `"`
  * `{{p}}` -> `|`
