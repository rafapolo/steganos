require 'oily_png'
require 'base64'
require 'pry'
require 'zlib'

class String
  def to_hex
    self.unpack('H*')[0]
  end

  def from_hex
    [self].pack('H*')
  end

  def to_b64
  	Base64.encode64(self)
  end

  def compact
	Zlib::Deflate.deflate(self, Zlib::BEST_COMPRESSION)
  end
end

def is_int?(x)
	x % 1 == 0
end

def get_dimensions(size)	
	height = Math.sqrt(size)
	unless is_int? height
		width = size / height.ceil
		unless is_int? width
			width = width.floor
			height = height.ceil + 1
		end
	else
		width = height
	end
	[height.to_i, width.to_i]
end

def encode path
	puts "Encoding #{path}..."
	start = Time.now	
	data = File.read(path)	
	hex_encoded = data.to_b64.compact.to_hex
	size = hex_encoded.size/6+1
	dimension = get_dimensions(size)
	puts dimension
	height = dimension[0]
	width = dimension[1]

	puts "Pixels: #{size}"
	puts "Dimension: #{height}x#{width}"

	png = ChunkyPNG::Image.new(height, width)

	x = y = count = 0
	hex_encoded.scan(/.{1,6}/).each do |hexa_color|		
		printf("\r%d%", 100*(count+=1)/size+1)

		# fill last pixel missing data
		hexa_color = hexa_color + "0" * (6-hexa_color.size) if count>size-1 && hexa_color.size<6 
		png[x, y] = ChunkyPNG::Color.from_hex(hexa_color)

		# compose
		x += 1
		if x>height-1
			x = 0
			y += 1
		end
		if y>width-1
			y = 0
		end
		#puts "#{x}, #{y} = #{hexa_color}"
		
	end
	png.metadata['Author'] = 'ExtraPolo!'
	png.metadata['Title'] = path
	out_path = "#{path}.png"
	png.save(out_path, :interlace => true)
	
	original_size = data.size
	out_size = File.read(out_path).size

	puts ("-> encoded in %3.2f s" % [Time.now - start])
end

def decode path
	puts "Decoding #{path}..."
	start = Time.now
	png = ChunkyPNG::Image.from_file(path)	
	pixels = png.pixels
	size = pixels.size
	
	hex_encoded = ''
	count = 0
	pixels.each do |p|
		printf("\r%d%", 100*(count+=1)/size)
		hex_encoded << ChunkyPNG::Color.to_hex(p)	
	end

	puts
	hex_encoded.gsub!('#', '')
	puts hex_encoded

	original_data = Base64.decode64(hex_encoded.from_hex)
	original_title = png.metadata['Title']
	File.open("out-#{original_title}", 'w'){|f| f.puts original_data}
	puts ("-> decoded \"#{original_title}\" in %3.2f s" % [Time.now - start])
end

system "clear"
encode 'cypherpunks.pdf'
decode 'Ostrom - Governing the Commons - The Evolution of Institutions for Collective Action.pdf.png'