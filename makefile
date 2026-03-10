build:
	gcc payjar.c -o pjc
	ln -s pjc pjrt
clean:
	rm pjc pjrt
update: clean build
test:
	echo "not implemented"

