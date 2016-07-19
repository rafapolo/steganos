fs = require "fs"
PNG = require("pngjs").PNG

encode = (input) ->
  fs.readFile input, (err, data) ->
    data = new Buffer(data, 'binary')
    console.log data
    
    size = data.length
    side = Math.ceil (Math.sqrt size)
    console.log "#{size} pixels"
    console.log "dimension: #{side} x #{side}"

    image = new PNG(width: side, height: side)
    image.data = data
    console.log '--- encoding ---'
    image.pack().pipe(fs.createWriteStream("#{input}.out.png"))


decode = (input, output) -> 
    fs.createReadStream(input).pipe(new PNG()).on "parsed", ->      
      console.log '--- decoding ---'
      result = new Buffer(@data.slice(0, (@height*@width) - 5))
      console.log result
      console.log result.length
      fs.writeFile output, result, (err) -> console.log "Ok!" unless err


#encode('bia.jpg')
#decode('bia.jpg.out.png', 'bia.out.jpg')