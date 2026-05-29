<gap:target id="checkout-form-component">
  <gap:target id="types-section">
    export interface <gap:target id="form-data-type-name">FormData</gap:target> {
      <gap:target id="full-name-field">fullName</gap:target>: string;
      <gap:target id="email-field">email</gap:target>: string;
      <gap:target id="address-field">address</gap:target>: string;
      <gap:target id="city-field">city</gap:target>: string;
      <gap:target id="state-field">state</gap:target>: string;
      <gap:target id="zip-field">zip</gap:target>: string;
      <gap:target id="country-field">country</gap:target>: string;
      <gap:target id="shipping-method-field">shippingMethod</gap:target>: string;
      <gap:target id="card-name-field">cardName</gap:target>: string;
      <gap:target id="card-number-field">cardNumber</gap:target>: string;
      <gap:target id="card-expiry-field">cardExpiry</gap:target>: string;
      <gap:target id="card-cvv-field">cardCvv</gap:target>: string;
    }

    export interface <gap:target id="validation-errors-type-name">ValidationErrors</gap:target> {
      <gap:target id="validation-full-name-key">fullName</gap:target>?: string;
      <gap:target id="validation-email-key">email</gap:target>?: string;
      <gap:target id="validation-address-key">address</gap:target>?: string;
      <gap:target id="validation-city-key">city</gap:target>?: string;
      <gap:target id="validation-state-key">state</gap:target>?: string;
      <gap:target id="validation-zip-key">zip</gap:target>?: string;
      <gap:target id="validation-country-key">country</gap:target>?: string;
      <gap:target id="validation-shipping-method-key">shippingMethod</gap:target>?: string;
      <gap:target id="validation-card-name-key">cardName</gap:target>?: string;
      <gap:target id="validation-card-number-key">cardNumber</gap:target>?: string;
      <gap:target id="validation-card-expiry-key">cardExpiry</gap:target>?: string;
      <gap:target id="validation-card-cvv-key">cardCvv</gap:target>?: string;
      <gap:target id="validation-form-key">form</gap:target>?: string;
    }

    export interface <gap:target id="cart-item-type-name">CartItem</gap:target> {
      <gap:target id="cart-item-id-field">id</gap:target>: string;
      <gap:target id="cart-item-name-field">name</gap:target>: string;
      <gap:target id="cart-item-price-field">price</gap:target>: number;
      <gap:target id="cart-item-quantity-field">quantity</gap:target>: number;
    }

    export interface <gap:target id="shipping-option-type-name">ShippingOption</gap:target> {
      <gap:target id="shipping-option-id-field">id</gap:target>: string;
      <gap:target id="shipping-option-label-field">label</gap:target>: string;
      <gap:target id="shipping-option-price-field">price</gap:target>: number;
      <gap:target id="shipping-option-days-field">days</gap:target>: string;
    }
  </gap:target>

  <gap:target id="validation-section">
    <gap:target id="required-field-validator-name">required</gap:target> = (value: string, fieldName: string): string | null => {
      return value.trim() ? null : `${fieldName} is required.`;
    };

    <gap:target id="email-validator-name">validateEmail</gap:target> = (email: string): string | null => {
      const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
      return emailRegex.test(email.trim()) ? null : "Please enter a valid email address.";
    };

    <gap:target id="luhn-validator-name">validateCreditCard</gap:target> = (cardNumber: string): string | null => {
      const digits = cardNumber.replace(/\D/g, "");
      if (!digits) return "Card number is required.";
      let sum = 0;
      let shouldDouble = false;

      for (let i = digits.length - 1; i >= 0; i--) {
        let digit = parseInt(digits[i], 10);
        if (shouldDouble) {
          digit *= 2;
          if (digit > 9) digit -= 9;
        }
        sum += digit;
        shouldDouble = !shouldDouble;
      }

      return sum % 10 === 0 ? null : "Please enter a valid credit card number.";
    };

    <gap:target id="expiry-validator-name">validateExpiryDate</gap:target> = (expiry: string): string | null => {
      const match = expiry.match(/^(\d{2})\/(\d{2})$/);
      if (!match) return "Use MM/YY format.";

      const month = parseInt(match[1], 10);
      const year = parseInt(match[2], 10);
      if (month < 1 || month > 12) return "Invalid expiry month.";

      const now = new Date();
      const currentYear = now.getFullYear() % 100;
      const currentMonth = now.getMonth() + 1;

      if (year < currentYear || (year === currentYear && month < currentMonth)) {
        return "Card has expired.";
      }

      return null;
    };
  </gap:target>

  <gap:target id="form-field-components-section">
    const <gap:target id="text-input-component-name">TextInput</gap:target>: React.FC<{
      <gap:target id="text-input-label-prop">label</gap:target>: string;
      <gap:target id="text-input-name-prop">name</gap:target>: string;
      <gap:target id="text-input-type-prop">type</gap:target>?: string;
      <gap:target id="text-input-value-prop">value</gap:target>: string;
      <gap:target id="text-input-onchange-prop">onChange</gap:target>: (e: React.ChangeEvent<HTMLInputElement>) => void;
      <gap:target id="text-input-error-prop">error</gap:target>?: string;
      <gap:target id="text-input-placeholder-prop">placeholder</gap:target>?: string;
    }> = ({ label, name, type = "text", value, onChange, error, placeholder }) => (
      <div style={styles.field}>
        <label style={styles.label} htmlFor={name}>{label}</label>
        <input
          id={name}
          name={name}
          type={type}
          value={value}
          onChange={onChange}
          placeholder={placeholder}
          style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
        />
        {error && <div style={styles.errorText}>{error}</div>}
      </div>
    );

    const <gap:target id="select-component-name">SelectInput</gap:target>: React.FC<{
      <gap:target id="select-label-prop">label</gap:target>: string;
      <gap:target id="select-name-prop">name</gap:target>: string;
      <gap:target id="select-value-prop">value</gap:target>: string;
      <gap:target id="select-onchange-prop">onChange</gap:target>: (e: React.ChangeEvent<HTMLSelectElement>) => void;
      <gap:target id="select-options-prop">options</gap:target>: { value: string; label: string }[];
      <gap:target id="select-error-prop">error</gap:target>?: string;
    }> = ({ label, name, value, onChange, options, error }) => (
      <div style={styles.field}>
        <label style={styles.label} htmlFor={name}>{label}</label>
        <select
          id={name}
          name={name}
          value={value}
          onChange={onChange}
          style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
        >
          {options.map(<gap:target id="select-option-render-function">option</gap:target> => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        {error && <div style={styles.errorText}>{error}</div>}
      </div>
    );

    const <gap:target id="card-number-component-name">CardNumberInput</gap:target>: React.FC<{
      <gap:target id="card-number-label-prop">label</gap:target>: string;
      <gap:target id="card-number-name-prop">name</gap:target>: string;
      <gap:target id="card-number-value-prop">value</gap:target>: string;
      <gap:target id="card-number-onchange-prop">onChange</gap:target>: (value: string) => void;
      <gap:target id="card-number-error-prop">error</gap:target>?: string;
    }> = ({ label, name, value, onChange, error }) => {
      const formatCardNumber = (input: string) => {
        const digits = input.replace(/\D/g, "").slice(0, 16);
        return digits.replace(/(.{4})/g, "$1 ").trim();
      };

      return (
        <div style={styles.field}>
          <label style={styles.label} htmlFor={name}>{label}</label>
          <input
            id={name}
            name={name}
            type="text"
            inputMode="numeric"
            autoComplete="cc-number"
            value={value}
            onChange={(e) => onChange(formatCardNumber(e.target.value))}
            placeholder="1234 5678 9012 3456"
            style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
          />
          {error && <div style={styles.errorText}>{error}</div>}
        </div>
      );
    };

    const <gap:target id="expiry-picker-component-name">ExpiryPicker</gap:target>: React.FC<{
      <gap:target id="expiry-picker-label-prop">label</gap:target>: string;
      <gap:target id="expiry-picker-name-prop">name</gap:target>: string;
      <gap:target id="expiry-picker-value-prop">value</gap:target>: string;
      <gap:target id="expiry-picker-onchange-prop">onChange</gap:target>: (value: string) => void;
      <gap:target id="expiry-picker-error-prop">error</gap:target>?: string;
    }> = ({ label, name, value, onChange, error }) => (
      <div style={styles.field}>
        <label style={styles.label} htmlFor={name}>{label}</label>
        <input
          id={name}
          name={name}
          type="text"
          inputMode="numeric"
          autoComplete="cc-exp"
          value={value}
          onChange={(e) => {
            const raw = e.target.value.replace(/\D/g, "").slice(0, 4);
            const formatted = raw.length >= 3 ? `${raw.slice(0, 2)}/${raw.slice(2)}` : raw;
            onChange(formatted);
          }}
          placeholder="MM/YY"
          style={{ ...styles.input, ...(error ? styles.inputError : {}) }}
        />
        {error && <div style={styles.errorText}>{error}</div>}
      </div>
    );
  </gap:target>

  <gap:target id="checkout-component-section">
    import React, { useMemo, useState } from "react";

    const <gap:target id="shipping-options-const-name">shippingOptions</gap:target>: ShippingOption[] = [
      { id: "standard", label: "Standard Shipping", price: 5.99, days: "5-7 business days" },
      { id: "express", label: "Express Shipping", price: 14.99, days: "2-3 business days" },
      { id: "overnight", label: "Overnight Shipping", price: 29.99, days: "1 business day" },
    ];

    const <gap:target id="initial-form-state-name">initialFormData</gap:target>: FormData = {
      fullName: "",
      email: "",
      address: "",
      city: "",
      state: "",
      zip: "",
      country: "US",
      shippingMethod: "standard",
      cardName: "",
      cardNumber: "",
      cardExpiry: "",
      cardCvv: "",
    };

    export const <gap:target id="checkout-component-name">CheckoutForm</gap:target>: React.FC<{
      <gap:target id="checkout-cart-items-prop">cartItems</gap:target>: CartItem[];
      <gap:target id="checkout-on-submit-prop">onSubmitOrder</gap:target>: (data: FormData) => Promise<void>;
    }> = ({ cartItems, onSubmitOrder }) => {
      const [formData, setFormData] = useState<FormData>(initialFormData);
      const [errors, setErrors] = useState<ValidationErrors>({});
      const [loading, setLoading] = useState<boolean>(false);
      const [submitError, setSubmitError] = useState<string>("");

      const selectedShipping = useMemo(
        () => shippingOptions.find((option) => option.id === formData.shippingMethod) ?? shippingOptions[0],
        [formData.shippingMethod]
      );

      const subtotal = useMemo(
        () => cartItems.reduce((sum, item) => sum + item.price * item.quantity, 0),
        [cartItems]
      );
      const shipping = selectedShipping.price;
      const tax = subtotal * 0.08;
      const total = subtotal + shipping + tax;

      const updateField = <K extends keyof FormData>(field: K, value: FormData[K]) => {
        setFormData((prev) => ({ ...prev, [field]: value }));
      };

      const validate = (): ValidationErrors => {
        const nextErrors: ValidationErrors = {};
        const requiredFields: (keyof FormData)[] = ["fullName", "email", "address", "city", "state", "zip", "country", "shippingMethod", "cardName", "cardNumber", "cardExpiry", "cardCvv"];

        requiredFields.forEach((field) => {
          const message = required(formData[field], String(field));
          if (message) nextErrors[field] = message;
        });

        const emailError = validateEmail(formData.email);
        if (emailError) nextErrors.email = emailError;

        const cardError = validateCreditCard(formData.cardNumber);
        if (cardError) nextErrors.cardNumber = cardError;

        const expiryError = validateExpiryDate(formData.cardExpiry);
        if (expiryError) nextErrors.cardExpiry = expiryError;

        if (formData.cardCvv.replace(/\D/g, "").length < 3) {
          nextErrors.cardCvv = "Please enter a valid CVV.";
        }

        return nextErrors;
      };

      const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setSubmitError("");
        const nextErrors = validate();
        setErrors(nextErrors);
        if (Object.keys(nextErrors).length > 0) return;

        try {
          setLoading(true);
          await onSubmitOrder(formData);
        } catch (error) {
          setSubmitError(error instanceof Error ? error.message : "Something went wrong. Please try again.");
        } finally {
          setLoading(false);
        }
      };

      return (
        <div style={styles.page}>
          <div style={styles.container}>
            <form onSubmit={handleSubmit} style={styles.form}>
              <div style={styles.sectionHeader}>
                <h2 style={styles.title}>Checkout</h2>
                <p style={styles.subtitle}>Complete your purchase securely.</p>
              </div>

              <div style={styles.section}>
                <h3 style={styles.sectionTitle}>Contact & Shipping</h3>
                <TextInput label="Full Name" name="fullName" value={formData.fullName} onChange={(e) => updateField("fullName", e.target.value)} error={errors.fullName} />
                <TextInput label="Email" name="email" type="email" value={formData.email} onChange={(e) => updateField("email", e.target.value)} error={errors.email} />
                <TextInput label="Address" name="address" value={formData.address} onChange={(e) => updateField("address", e.target.value)} error={errors.address} />
                <div style={styles.row}>
                  <TextInput label="City" name="city" value={formData.city} onChange={(e) => updateField("city", e.target.value)} error={errors.city} />
                  <TextInput label="State" name="state" value={formData.state} onChange={(e) => updateField("state", e.target.value)} error={errors.state} />
                </div>
                <div style={styles.row}>
                  <TextInput label="ZIP" name="zip" value={formData.zip} onChange={(e) => updateField("zip", e.target.value)} error={errors.zip} />
                  <SelectInput
                    label="Country"
                    name="country"
                    value={formData.country}
                    onChange={(e) => updateField("country", e.target.value)}
                    options={[
                      { value: "US", label: "United States" },
                      { value: "CA", label: "Canada" },
                      { value: "GB", label: "United Kingdom" },
                    ]}
                    error={errors.country}
                  />
                </div>
                <SelectInput
                  label="Shipping Method"
                  name="shippingMethod"
                  value={formData.shippingMethod}
                  onChange={(e) => updateField("shippingMethod", e.target.value)}
                  options={shippingOptions.map((option) => ({
                    value: option.id,
                    label: `${option.label} — $${option.price.toFixed(2)} (${option.days})`,
                  }))}
                  error={errors.shippingMethod}
                />
              </div>

              <div style={styles.section}>
                <h3 style={styles.sectionTitle}>Payment</h3>
                <TextInput label="Cardholder Name" name="cardName" value={formData.cardName} onChange={(e) => updateField("cardName", e.target.value)} error={errors.cardName} />
                <CardNumberInput label="Card Number" name="cardNumber" value={formData.cardNumber} onChange={(value) => updateField("cardNumber", value)} error={errors.cardNumber} />
                <div style={styles.row}>
                  <ExpiryPicker label="Expiry Date" name="cardExpiry" value={formData.cardExpiry} onChange={(value) => updateField("cardExpiry", value)} error={errors.cardExpiry} />
                  <TextInput label="CVV" name="cardCvv" value={formData.cardCvv} onChange={(e) => updateField("cardCvv", e.target.value.replace(/\D/g, "").slice(0, 4))} error={errors.cardCvv} />
                </div>
              </div>

              {submitError && <div style={styles.submitError}>{submitError}</div>}

              <button type="submit" disabled={loading} style={{ ...styles.button, ...(loading ? styles.buttonDisabled : {}) }}>
                {loading ? "Processing..." : "Place Order"}
              </button>
            </form>

            <aside style={styles.sidebar}>
              <h3 style={styles.sectionTitle}>Order Summary</h3>
              <div style={styles.summaryList}>
                {cartItems.map((item) => (
                  <div key={item.id} style={styles.summaryItem}>
                    <div>
                      <div style={styles.summaryName}>{item.name}</div>
                      <div style={styles.summaryMeta}>Qty {item.quantity}</div>
                    </div>
                    <div style={styles.summaryPrice}>${(item.price * item.quantity).toFixed(2)}</div>
                  </div>
                ))}
              </div>

              <div style={styles.totals}>
                <div style={styles.totalRow}><span>Subtotal</span><span>${subtotal.toFixed(2)}</span></div>
                <div style={styles.totalRow}><span>Shipping</span><span>${shipping.toFixed(2)}</span></div>
                <div style={styles.totalRow}><span>Tax</span><span>${tax.toFixed(2)}</span></div>
                <div style={{ ...styles.totalRow, ...styles.grandTotal }}><span>Total</span><span>${total.toFixed(2)}</span></div>
              </div>
            </aside>
          </div>
        </div>
      );
    };

    const styles: Record<string, React.CSSProperties> = {
      page: {
        minHeight: "100vh",
        background: "#f6f7fb",
        padding: "32px",
        boxSizing: "border-box",
        fontFamily: "Arial, sans-serif",
      },
      container: {
        maxWidth: "1200px",
        margin: "0 auto",
        display: "grid",
        gridTemplateColumns: "2fr 1fr",
        gap: "24px",
        alignItems: "start",
      },
      form: {
        background: "#fff",
        borderRadius: "16px",
        padding: "24px",
        boxShadow: "0 8px 30px rgba(0,0,0,0.08)",
      },
      sidebar: {
        background: "#fff",
        borderRadius: "16px",
        padding: "24px",
        boxShadow: "0 8px 30px rgba(0,0,0,0.08)",
        position: "sticky",
        top: "24px",
      },
      sectionHeader: { marginBottom: "24px" },
      title: { margin: 0, fontSize: "28px", color: "#111827" },
      subtitle: { margin: "8px 0 0", color: "#6b7280" },
      section: { marginBottom: "24px" },
      sectionTitle: { margin: "0 0 16px", fontSize: "18px", color: "#111827" },
      field: { marginBottom: "16px" },
      label: { display: "block", marginBottom: "8px", fontWeight: 600, color: "#374151" },
      input: {
        width: "100%",
        padding: "12px 14px",
        borderRadius: "10px",
        border: "1px solid #d1d5db",
        outline: "none",
        fontSize: "14px",
        boxSizing: "border-box",
      },
      inputError: { borderColor: "#ef4444", background: "#fff5f5" },
      errorText: { marginTop: "6px", color: "#ef4444", fontSize: "13px" },
      row: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" },
      submitError: {
        marginBottom: "16px",
        padding: "12px 14px",
        borderRadius: "10px",
        background: "#fef2f2",
        color: "#b91c1c",
        border: "1px solid #fecaca",
      },
      button: {
        width: "100%",
        padding: "14px 16px",
        borderRadius: "12px",
        border: "none",
        background: "#2563eb",
        color: "#fff",
        fontSize: "16px",
        fontWeight: 700,
        cursor: "pointer",
      },
      buttonDisabled: { opacity: 0.7, cursor: "not-allowed" },
      summaryList: { display: "grid", gap: "12px", marginBottom: "20px" },
      summaryItem: {
        display: "flex",
        justifyContent: "space-between",
        gap: "12px",
        paddingBottom: "12px",
        borderBottom: "1px solid #e5e7eb",
      },
      summaryName: { fontWeight: 600, color: "#111827" },
      summaryMeta: { fontSize: "13px", color: "#6b7280", marginTop: "4px" },
      summaryPrice: { fontWeight: 600, color: "#111827" },
      totals: {
        display: "grid",
        gap: "10px",
        paddingTop: "8px",
        borderTop: "1px solid #e5e7eb",
      },
      totalRow: { display: "flex", justifyContent: "space-between", color: "#374151" },
      grandTotal: {
        fontSize: "18px",
        fontWeight: 800,
        color: "#111827",
        paddingTop: "8px",
      },
    };
  </gap:target>
</gap:target>