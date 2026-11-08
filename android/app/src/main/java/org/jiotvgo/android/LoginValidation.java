package org.jiotvgo.android;

import java.util.regex.Pattern;

final class LoginValidation {
    private static final Pattern TEN_DIGITS = Pattern.compile("[0-9]{10}");
    private static final Pattern SIX_DIGITS = Pattern.compile("[0-9]{6}");
    private static final Pattern INDIAN_INTERNATIONAL = Pattern.compile("\\+91[0-9]{10}");

    private LoginValidation() {}

    static boolean isValidMobile(String value) {
        if (value == null) return false;
        String mobile = value.trim();
        return TEN_DIGITS.matcher(mobile).matches() || INDIAN_INTERNATIONAL.matcher(mobile).matches();
    }

    static String normalizeIndianMobile(String value) {
        if (value == null) throw new IllegalArgumentException("Mobile number is required");
        String mobile = value.trim();
        if (TEN_DIGITS.matcher(mobile).matches()) return "+91" + mobile;
        if (INDIAN_INTERNATIONAL.matcher(mobile).matches()) return mobile;
        throw new IllegalArgumentException("Enter a 10-digit Indian mobile number");
    }

    static boolean isValidOtp(String value) {
        return value != null && SIX_DIGITS.matcher(value.trim()).matches();
    }
}
