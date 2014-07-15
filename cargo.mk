ifneq ($(findstring i686,$(TARGET)),)
export CC := $(CC) -m32
endif

export COPT := -fPIC

all:
	$(MAKE) -C src/civetweb lib BUILD_DIR=$(DEPS_DIR)
	mv src/civetweb/libcivetweb.a $(DEPS_DIR)/
