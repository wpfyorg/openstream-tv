package org.jiotvgo.android;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class LoginValidationTest {
    @Test public void acceptsAndNormalizesTenDigitIndianMobile() {
        assertTrue(LoginValidation.isValidMobile("9120031376"));
        assertTrue(LoginValidation.isValidMobile(" 9120031376 "));
        assertEquals("+919120031376", LoginValidation.normalizeIndianMobile("9120031376"));
    }

    @Test public void acceptsAlreadyInternationalIndianMobile() {
        assertTrue(LoginValidation.isValidMobile("+919120031376"));
        assertEquals("+919120031376", LoginValidation.normalizeIndianMobile("+919120031376"));
    }

    @Test public void rejectsInvalidMobileValues() {
        assertFalse(LoginValidation.isValidMobile(null));
        assertFalse(LoginValidation.isValidMobile("912003137"));
        assertFalse(LoginValidation.isValidMobile("91200313767"));
        assertFalse(LoginValidation.isValidMobile("+441234567890"));
        assertFalse(LoginValidation.isValidMobile("91 20031376"));
    }

    @Test public void requiresExactlySixNumericOtpDigits() {
        assertTrue(LoginValidation.isValidOtp("123456"));
        assertTrue(LoginValidation.isValidOtp(" 123456 "));
        assertFalse(LoginValidation.isValidOtp(null));
        assertFalse(LoginValidation.isValidOtp("12345"));
        assertFalse(LoginValidation.isValidOtp("1234567"));
        assertFalse(LoginValidation.isValidOtp("12a456"));
    }
}
