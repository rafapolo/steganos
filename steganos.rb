# https://github.com/rafapolo/steganos

require 'oily_png'
require 'base64'
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

  def from_b64
  	Base64.decode64(self)
  end

  def zip
	Zlib::Deflate.deflate(self, Zlib::BEST_COMPRESSION)
  end

  def unzip
	Zlib::Inflate.inflate(self)
  end
end

class Steganos

	# get aproximated better dimension for needed pixels size
	# 15 = 3 x 5 not 4 x 4 as follow
	def get_dimensions(size)
		dim = Math.sqrt(size).ceil
		[dim, dim]

		# experimental:
		# height = Math.sqrt(size)
		# unless height % 1 == 0 # is integer ?
		# 	width = size / height.ceil
		# 	unless width % 1 == 0
		# 		width = width.floor
		# 		height = height.ceil + 1
		# 	end
		# else
		# 	width = height
		# end
		# [height.to_i, width.to_i]
	end

	def encode path
		puts "Encoding #{path}..."
		start = Time.now

		data = File.read(path)
		hex_encoded = data.to_b64.zip.to_hex

		size = hex_encoded.size/6+1
		dimension = get_dimensions(size)
		height = dimension[0]
		width = dimension[1]

		puts "Pixels: #{size}"
		puts "Dimension: #{height}x#{width}"

		png = ChunkyPNG::Image.new(height, width)
		x = y = count = 0
		hex_encoded.scan(/.{1,6}/).each do |hexa_color|
			printf("\r%d%", 100*(count+=1)/size)
			# fill last pixel missing data
			hexa_color = hexa_color + "0" * (6-hexa_color.size) if count>size-1 && hexa_color.size<6
			# compose image
			png[x, y] = ChunkyPNG::Color.from_hex(hexa_color)
			#puts "[#{x}, #{y}] => #{hexa_color}"
			x += 1
			if x>height-1
				x = 0
				y += 1
			end
			y = 0 if y>width

		end
		png.metadata['Author'] = 'ExtraPolo!'
		png.metadata['Title'] = path
		out_path = "#{path}.png"
		png.save(out_path, :interlace => true)

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
			if match_hex = ChunkyPNG::Color.to_hex(p).match(/#(.{1,6})ff/)
				clean_hex = match_hex[1]
				# remove extra data on last pixel
				clean_hex.gsub!(/0+$/, '') if count==size
				hex_encoded << clean_hex
			end
		end

		# inverse data.to_b64.zip.to_hex
		original_data = hex_encoded.from_hex.unzip.from_b64

		original_title = png.metadata['Title']
		File.open("out-#{original_title}", 'w'){|f| f.puts original_data}
		puts ("-> decoded \"#{original_title}\" in %3.2f s" % [Time.now - start])
	end
end

system "clear"

stegano = Steganos.new
stegano.decode 'cypherpunks.pdf.png'
