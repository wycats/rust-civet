all:
	$(MAKE) -C src/civetweb lib
	mv src/civetweb/libcivetweb.a $(DEPS_DIR)/
