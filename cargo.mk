all:
	$(MAKE) -C src/civetweb lib BUILD_DIR=$(DEPS_DIR)
	mv src/civetweb/libcivetweb.a $(DEPS_DIR)/
