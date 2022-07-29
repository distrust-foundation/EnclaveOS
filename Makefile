DEBUG := false
OUT_DIR := out
KEY_DIR := keys
SRC_DIR := src
TARGET := local
CACHE_DIR := cache
CONFIG_DIR := config
TOOLCHAIN_DIR := config/toolchain
SRC_DIR := src
USER := $(shell id -g):$(shell id -g)
CPUS := $(shell nproc)
ARCH := x86_64

include $(PWD)/config/global.env
include $(TOOLCHAIN_DIR)/Makefile

.DEFAULT_GOAL := default
.PHONY: default
default: fetch $(OUT_DIR)/bzImage

# Clean repo back to initial clone state
.PHONY: clean
clean:
	rm -rf cache out
	docker image rm -f local/$(NAME)-build

# Launch a shell inside the toolchain container
.PHONY: toolchain-shell
toolchain-shell: $(OUT_DIR)/toolchain.tar
	$(call toolchain,root,bash)

# Pin all packages in toolchain container to latest versions
.PHONY: toolchain-update
toolchain-update:
	$(call toolchain,root,toolchain-update )

# Source anything required from the internet to build
.PHONY: fetch
fetch: \
	keys \
	$(OUT_DIR) \
	$(OUT_DIR)/toolchain.tar \
	$(CACHE_DIR) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig

# Build latest image and run in terminal via Qemu
.PHONY: run
run: default
	qemu-system-x86_64 \
		-m 512M \
		-nographic \
		-kernel $(OUT_DIR)/bzImage

# Run ncurses busybox config menu and save output
.PHONY: busybox-config
busybox-config:
	rm $(CONFIG_DIR)/debug/busybox.config
	make $(CONFIG_DIR)/debug/busybox.config

# Run linux config menu and save output
.PHONY: linux-config
linux-config:
	rm $(CONFIG_DIR)/$(TARGET)/linux.config
	make $(CONFIG_DIR)/$(TARGET)/linux.config

.PHONY: keys
keys: \
	$(KEY_DIR)/$(LINUX_KEY).asc \
	$(KEY_DIR)/$(BUSYBOX_KEY).asc

$(KEY_DIR)/$(LINUX_KEY).asc:
	$(call fetch_pgp_key,$(LINUX_KEY))

$(KEY_DIR)/$(BUSYBOX_KEY).asc:
	$(call fetch_pgp_key,,$(BUSYBOX_KEY))

define fetch_pgp_key
	mkdir -p $(KEY_DIR) && \
	$(toolchain) ' \
		for server in \
    	    ha.pool.sks-keyservers.net \
    	    hkp://keyserver.ubuntu.com:80 \
    	    hkp://p80.pool.sks-keyservers.net:80 \
    	    pgp.mit.edu \
    	; do \
			echo "Trying: $${server}"; \
    	   	gpg \
    	   		--recv-key \
    	   		--keyserver "$${server}" \
    	   		--keyserver-options timeout=10 \
    	   		--recv-keys "$(1)" \
    	   	&& break; \
    	done; \
		gpg --export -a $(1) > $(KEY_DIR)/$(1).asc; \
	'
endef

$(OUT_DIR):
	mkdir -p $(OUT_DIR)

$(CACHE_DIR):
	mkdir -p $(CACHE_DIR)

$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig:
	curl \
		--url $(BUSYBOX_SERVER)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig \
		--output $(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig

$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2:
	curl \
		--url $(BUSYBOX_SERVER)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
		--output $(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2

$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign:
	curl \
		--url $(LINUX_SERVER)/linux-$(LINUX_VERSION).tar.sign \
		--output $(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign

$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz:
	curl \
		--url $(LINUX_SERVER)/linux-$(LINUX_VERSION).tar.xz \
		--output $(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz

$(CACHE_DIR)/linux-$(LINUX_VERSION).tar:
	xz -d $(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz

$(CACHE_DIR)/linux-$(LINUX_VERSION): $(CACHE_DIR)/linux-$(LINUX_VERSION).tar
	$(call toolchain,$(USER), " \
		cd /cache && \
		gpg --import /keys/$(LINUX_KEY).asc && \
		gpg --verify linux-$(LINUX_VERSION).tar.sign && \
		tar xf linux-$(LINUX_VERSION).tar; \
	")

$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION):
	$(call toolchain,$(USER), " \
		cd /cache && \
		gpg --import /keys/$(BUSYBOX_KEY).asc && \
		gpg --verify busybox-$(BUSYBOX_VERSION).tar.bz2.sig && \
		tar -xf busybox-$(BUSYBOX_VERSION).tar.bz2 \
	")

# This can likely be eliminated with path fixes in toolchain/Makefile
$(OUT_DIR)/toolchain.tar:
	ARCH=$(ARCH) \
	OUT_DIR=../../$(OUT_DIR) \
	DEBIAN_HASH=$(DEBIAN_HASH) \
	$(MAKE) -C $(TOOLCHAIN_DIR) \
	../../$(OUT_DIR)/toolchain.tar

$(CONFIG_DIR)/debug/busybox.config:
	$(call toolchain,$(USER), " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		KCONFIG_NOTIMESTAMP=1 make menuconfig && \
		cp .config /config/debug/busybox.config; \
	")

$(CONFIG_DIR)/$(TARGET)/linux.config:
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		make menuconfig && \
		cp .config /config/$(TARGET)/linux.config; \
	")

$(OUT_DIR)/busybox: \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION) \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig
	$(call toolchain,$(USER)," \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		cp /config/debug/busybox.config .config && \
		make -j$(CPUS) busybox && \
		cp busybox /out/; \
	")

$(OUT_DIR)/init:
	$(call toolchain,$(USER)," \
		gcc \
			-static \
			-static-libgcc /src/init.c \
			-o /out/init; \
	")

$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio: \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		gcc usr/gen_init_cpio.c -o usr/gen_init_cpio \
	")

$(OUT_DIR)/rootfs.cpio: \
	$(OUT_DIR)/busybox \
	$(OUT_DIR)/init \
	$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio
	mkdir -p $(CACHE_DIR)/rootfs/bin
ifeq ($(DEBUG), true)
	cp $(OUT_DIR)/init $(CACHE_DIR)/rootfs/real_init
	cp $(SRC_DIR)/scripts/busybox_init $(CACHE_DIR)/rootfs/init
	cp $(OUT_DIR)/busybox $(CACHE_DIR)/rootfs/bin/
else
	cp $(OUT_DIR)/init $(CACHE_DIR)/rootfs/init
endif
	$(call toolchain,$(USER)," \
		cd /cache/rootfs && \
		find . -mindepth 1 -execdir touch -hcd "@0" "{}" + && \
		find . -mindepth 1 -printf '%P\0' && \
		cd /cache/linux-$(LINUX_VERSION) && \
		usr/gen_initramfs.sh \
			-o /out/rootfs.cpio \
			/config/$(TARGET)/rootfs.list && \
		cpio -itv < /out/rootfs.cpio && \
		sha256sum /out/rootfs.cpio; \
	")

$(OUT_DIR)/bzImage: \
	$(OUT_DIR)/rootfs.cpio
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		cp /config/$(TARGET)/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		cp arch/x86_64/boot/bzImage /out/ && \
		sha256sum /out/bzImage; \
	")
