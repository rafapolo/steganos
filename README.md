Steganos
=======

Algorithms to steganography files into images, and vice-versa.

http://en.wikipedia.org/wiki/Steganography

The concept is easy and creative: get the data from the file, encode into base64, compact and encode into hexadecimal values, so get each three chunks of hexa values to compose the pixels colors that represents the file as an encoded PNG image.

![1.3mb encoded PDF and zoom](https://raw.github.com/rafapolo/steganos/master/sample.png)

This was a prove of concept so I could upload 6GB of videos and PDFs to Flickr.