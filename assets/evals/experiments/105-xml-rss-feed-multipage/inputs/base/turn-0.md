Create a large, valid RSS 2.0 feed XML document for a technology news site called "The Daily Byte".

Channel metadata:
- title: The Daily Byte
- link: https://thedailybyte.example.com
- description: Technology news, reviews, and analysis updated throughout the day.
- language: en-us
- lastBuildDate: Wed, 27 May 2026 09:00:00 GMT
- generator: DailyByte Publisher 4.2

Generate EXACTLY 80 <item> entries, organized conceptually into 4 batches of 20 items each (items 1-20 = batch 1, items 21-40 = batch 2, items 41-60 = batch 3, items 61-80 = batch 4). The items are ordered newest-first, item 1 being the most recent.

Each <item> MUST contain, in this order:
- <title> a realistic technology headline
- <link> https://thedailybyte.example.com/articles/<n>  (where <n> is the item number 1-80)
- <guid isPermaLink="false"> a stable unique id of the form dailybyte-0001 ... dailybyte-0080 (zero-padded to 4 digits, matching the item number)
- <pubDate> an RFC-822 date, decreasing as the item number increases
- <category> one of: Hardware, Software, AI, Security, Mobile, Cloud
- <description> a one-sentence summary of the article

Use realistic, varied headlines, categories, and dates. Make every guid unique and stable. Output a single well-formed XML document beginning with <?xml version="1.0" encoding="UTF-8"?> and a root <rss version="2.0"> element containing one <channel>.