package org.opebstreamstv.android;

import android.app.Activity;
import android.graphics.Color;
import android.os.Bundle;
import android.view.View;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

/** Hosts the bundled OpenStream TV web Settings so providers can be configured on-device. */
public final class SettingsActivity extends Activity {
    static final String EXTRA_SETTINGS_URL = "org.opebstreamstv.android.SETTINGS_URL";
    private WebView web;

    @Override protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        String url = getIntent().getStringExtra(EXTRA_SETTINGS_URL);
        if (url == null || url.isEmpty()) {
            finish();
            return;
        }
        web = new WebView(this);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setSupportZoom(false);
        web.setBackgroundColor(Color.rgb(12, 14, 18));
        web.setWebViewClient(new WebViewClient());
        web.setFocusable(true);
        setContentView(web);
        web.loadUrl(url);
        web.requestFocus();
    }

    @Override public void onBackPressed() {
        if (web != null && web.canGoBack()) web.goBack();
        else super.onBackPressed();
    }

    @Override protected void onDestroy() {
        if (web != null) {
            web.loadUrl("about:blank");
            web.destroy();
            web = null;
        }
        super.onDestroy();
    }
}