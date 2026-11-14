include $(TOPDIR)/rules.mk

PKG_NAME:=openstream
PKG_VERSION?=1.6.0
PKG_RELEASE:=1
PKGARCH:=$(ARCH_PACKAGES)

include $(INCLUDE_DIR)/package.mk

define Package/openstream
  SECTION:=net
  CATEGORY:=Network
  TITLE:=OpenStream TV server with web UI
  URL:=https://github.com/wpfyorg/openstream-tv
  CONFLICTS:=openstream-slim
endef

define Package/openstream/description
 OpenStream TV multi-provider streaming and IPTV server, including the embedded web UI.
endef

define Package/openstream-slim
  SECTION:=net
  CATEGORY:=Network
  TITLE:=OpenStream TV headless server
  URL:=https://github.com/wpfyorg/openstream-tv
  CONFLICTS:=openstream
endef

define Package/openstream-slim/description
 Headless OpenStream TV server without the embedded web UI.
endef

define Build/Compile
endef

define Package/openstream/conffiles
/etc/config/openstream
endef

define Package/openstream-slim/conffiles
/etc/config/openstream
endef

define Package/openstream/install
	$(INSTALL_DIR) $(1)/usr/bin $(1)/etc/init.d $(1)/etc/config $(1)/etc/uci-defaults
	$(INSTALL_BIN) $(OPENSTREAM_FULL_BIN) $(1)/usr/bin/ost
	$(INSTALL_BIN) ./files/openstream.init $(1)/etc/init.d/openstream
	$(INSTALL_CONF) ./files/openstream.config $(1)/etc/config/openstream
	$(INSTALL_BIN) ./files/openstream.defaults $(1)/etc/uci-defaults/99-openstream
endef

define Package/openstream-slim/install
	$(INSTALL_DIR) $(1)/usr/bin $(1)/etc/init.d $(1)/etc/config $(1)/etc/uci-defaults
	$(INSTALL_BIN) $(OPENSTREAM_SLIM_BIN) $(1)/usr/bin/ost
	$(INSTALL_BIN) ./files/openstream.init $(1)/etc/init.d/openstream
	$(INSTALL_CONF) ./files/openstream.config $(1)/etc/config/openstream
	$(INSTALL_BIN) ./files/openstream.defaults $(1)/etc/uci-defaults/99-openstream
endef

$(eval $(call BuildPackage,openstream))
$(eval $(call BuildPackage,openstream-slim))
