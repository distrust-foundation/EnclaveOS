.PHONY: extract
extract: \
	fetch \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar \
	$(CACHE_DIR)/linux-$(LINUX_VERSION)/README \
    $(CACHE_DIR)/busybox-$(BUSYBOX_VERSION)/README

$(CACHE_DIR)/linux-$(LINUX_VERSION).tar:
	xz -d $(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz

$(CACHE_DIR)/linux-$(LINUX_VERSION)/README:
	$(toolchain) " \
		cd /cache && \
		gpg --import /keys/$(LINUX_KEY).asc && \
		gpg --verify linux-$(LINUX_VERSION).tar.sign && \
		tar xf linux-$(LINUX_VERSION).tar; \
	"

$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION)/README:
	$(toolchain) " \
		cd /cache && \
		gpg --import /keys/$(BUSYBOX_KEY).asc && \
		gpg --verify busybox-$(BUSYBOX_VERSION).tar.bz2.sig && \
		tar -xf busybox-$(BUSYBOX_VERSION).tar.bz2 \
	"
