Steganos
=======

The concept is easy and creative: get the data from the file, encode into base64, compact and encode it again into hexadecimal, then get each chunks of hexa values (ex: ff 12 d3) to compose the RGB pixels (ex: #ff12d3) that represents the data as an encoded PNG image in such a way that it can be reversed and decoded as the original file.

This was a prove of concept written in Ruby so I could upload 6GB of videos and PDFs to Flickr as images, and then get it back as the original files.

[Julian Assange, Cypherpunks 6.3MB PDF as a decodable 4MB PNG]


![Julian Assange - Cypherpunks PDF](https://github.com/rafapolo/steganos/blob/master/output-sample.jpg?raw=true)


Simple text encoding sample:

```
"extrapolo".to_b64.compact.to_hex
 => "78da8b8ac83048ce752b4f32aa28e302001fbe0464"

final 7 hexa color pixels to represent "extrapolo" data as image
 => #78da8b #8ac830 #48ce75 #2b4f32 #aa28e3 #02001f #be0464

and back to the original data,
"78da8b8ac83048ce752b4f32aa28e302001fbe0464".from_hex.unzip.from_b64
=> "extrapolo"

```

So, **"extrapolo"** as a PNG image:  
![](https://dummyimage.com/20x20/78da8b/78da8b.png&text=+) 
![](https://dummyimage.com/20x20/8ac830/8ac830.png&text=+) 
![](https://dummyimage.com/20x20/48ce75/48ce75.png&text=+) 
![](https://dummyimage.com/20x20/2b4f32/2b4f32.png&text=+) 
![](https://dummyimage.com/20x20/aa28e3/aa28e3.png&text=+) 
![](https://dummyimage.com/20x20/02001f/02001f.png&text=+) 
![](https://dummyimage.com/20x20/be0464/be0464.png&text=+)
<br>

More at: http://en.wikipedia.org/wiki/Steganography

--- 

##### -- 13 years later update --
### Rust version / AI Generated

  ```
  Encode: ~45.0× faster (1.9041 s / 0.0423 s)
  Decode: ~126.8× faster (1.0655 s / 0.0084 s)
```

A Rust implementation is included (now optimized) and can still decode the legacy Ruby PNGs.

Current encoding pipeline (optimized):

- zstd compression
- raw compressed bytes packed directly into RGB pixels (no base64/hex)
- tight rectangular dimensions (no forced square)
- PNG text metadata: Author, Title, PayloadLength
- RGB output (no alpha) for smaller images

Build and run:

```
cargo build --release
./targets/release/steganos
```

Encode any file into a PNG:

```
$ steganos encode path/to/input.bin
```

Sample:
```
  $ steganos encode cypherpunks.pdf
      - Encode time: 0.225 s
      - Encode sizes: input 6.261 MB, output 3.818 MB
  $ steganos decode out-cypherpunks.pdf.png
      - Decode time: 0.027 s
      - Decode sizes: input 3.818 MB, output 6.261 MB
```
