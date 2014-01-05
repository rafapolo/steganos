Steganos
=======

The concept is easy and creative: get the data from the file, encode into base64, compact and encode it again into hexadecimal, so get each three chunks of hexa values (ex: ff 12 d3) to compose the RGB pixels that represents the data as an encoded PNG image in such a way that you can reverse and decode it later as the original file.

This was a prove of concept so I could upload 6GB of videos and PDFs to Flickr as images.

[Julian Assange, Cypherpunks 6.3MB PDF as a decodable 4MB PNG]

![Julian Assange - Cypherpunks PDF](https://raw.github.com/rafapolo/steganos/master/cypherpunks.pdf.png)

Simple text encoding sample:

```
1) "extrapolo".to_b64
 => "ZXh0cmFwb2xv\n" 
 
2) "extrapolo".to_b64.compact
 => "x\xDA\x8B\x8A\xC80H\xCEu+O2\xAA(\xE3\x02\0\x1F\xBE\x04d" 
 
3) "extrapolo".to_b64.compact.to_hex
 => "78da8b8ac83048ce752b4f32aa28e302001fbe0464" 
 
4) final 7 hexa color pixels to represent "extrapolo" data as image
 => #78da8b #8ac830 #48ce75 #2b4f32 #aa28e3 #02001f #be0464
```

More: http://en.wikipedia.org/wiki/Steganography
