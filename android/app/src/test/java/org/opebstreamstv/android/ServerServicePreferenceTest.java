package org.opebstreamstv.android;

import android.content.Context;
import android.content.ContextWrapper;
import android.content.SharedPreferences;
import java.lang.reflect.Proxy;
import java.util.HashMap;
import java.util.Map;
import org.junit.Test;
import static org.junit.Assert.*;

public class ServerServicePreferenceTest {
    @Test public void bootStartPreferenceDefaultsOnAndPersistsExplicitChanges() {
        Map<String, Boolean> values = new HashMap<>();
        SharedPreferences preferences = (SharedPreferences) Proxy.newProxyInstance(
                SharedPreferences.class.getClassLoader(), new Class<?>[] { SharedPreferences.class },
                (proxy, method, args) -> {
                    if ("getBoolean".equals(method.getName())) {
                        return values.containsKey(args[0]) ? values.get(args[0]) : args[1];
                    }
                    if ("edit".equals(method.getName())) {
                        return Proxy.newProxyInstance(SharedPreferences.Editor.class.getClassLoader(),
                                new Class<?>[] { SharedPreferences.Editor.class }, (editor, editorMethod, editorArgs) -> {
                                    if ("putBoolean".equals(editorMethod.getName())) {
                                        values.put((String) editorArgs[0], (Boolean) editorArgs[1]);
                                        return editor;
                                    }
                                    if ("commit".equals(editorMethod.getName())) return true;
                                    return null;
                                });
                    }
                    return null;
                });
        String[] requestedPreferences = new String[1];
        Context context = new ContextWrapper(null) {
            @Override public SharedPreferences getSharedPreferences(String name, int mode) {
                requestedPreferences[0] = name;
                return preferences;
            }
        };

        assertTrue(ServerService.isBootStartEnabled(context));
        assertTrue(ServerService.setBootStartEnabled(context, false));
        assertFalse(ServerService.isBootStartEnabled(context));
        assertTrue(ServerService.setBootStartEnabled(context, true));
        assertTrue(ServerService.isBootStartEnabled(context));
        assertEquals("server_autostart", requestedPreferences[0]);
        assertEquals(Boolean.TRUE, values.get("enabled"));
    }
}
