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
