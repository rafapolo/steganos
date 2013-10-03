# encoding: utf-8
# autor: Rafael Polo
# data: 18.08.2013

# Prova de Conceito: É possível utilizar os incríveis 1024 gigabytes de armazenamento de imagens 
# do Flickr pra hospedar arquivos através de esteganografia nas imagens?

# esconde arquivo RAR em diversas imagens
def esteganografia(path)	
	out_path = "#{path}/out"
	# cria pasta temporária
	system("mkdir #{out_path}") unless File.directory? out_path

	puts "Compactando..."
	# Flickr tem o limite de 200MB por imagem, usarei 100MB por imagem.
	system("rar a -v100m #{out_path}/out.rar #{path}")

	# concateno RAR no fim do JPEG
	system("mkdir #{out_path}/images")
	Dir["#{out_path}/*.rar"].each do |chunk| 
		puts "Escondendo #{chunk}" # usa frame.jpg padrão
		system "cat #{path}/frame.jpg #{chunk} > #{chunk}.jpg"
	end
end

def upload_to_flickr
	# por hora fiz manualmente o upload das imagens em um novo Set de uma novo Usuário
	# http://www.flickr.com/photos/100332464@N03/sets/
	return set_id || 72157635116479848
end

def get_from_set(set_id)
	# uso API com key do novo usuário (poderia ser qualquer uma válida)
	require 'flickraw' # 3
	FlickRaw.api_key = '7fd9704ef650d1000263b1331277dd1'
	FlickRaw.shared_secret = 'c9ee9d15766fcb0'


	# pega as imagens das URLs de todas as imagens do Set dado
	threads = []
	flickr.photosets.getPhotos(:photoset_id => set_id).photo.each do |s|
		chunk = FlickRaw.url_o flickr.photos.getInfo(:photo_id => s.id)
		threads << Thread.new { system "wget #{chunk} -O #{s.title}" }
	end
	threads.each { |t| t.join }

	system "unrar e -y out.part01.rar"
	puts "Ok!"
end

# exemplo: cria 14 imagens distribuindo os 1.44 GB da pasta com o seguinte filme com legenda
esteganografia '/Users/polo/Downloads/Der.Himmel.Uber.Berlin.-.Wings.of.Desire.DVDRip.XviD.MakingOff.Org'
# sobe
set_id = upload_to_flickr
# remonta
get_from_set set_id
