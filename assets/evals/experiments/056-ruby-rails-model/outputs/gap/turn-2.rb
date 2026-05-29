<gap:target id="order-model-file">
class Order < ApplicationRecord
  <gap:target id="order-associations">
    belongs_to <gap:target id="user-association">:user</gap:target>
    has_many <gap:target id="order-items-association">:order_items</gap:target>
    has_one <gap:target id="shipping-address-association">:shipping_address</gap:target>
    has_many <gap:target id="payments-association">:payments    has_many <gap:target id="refunds-association">:refunds</gap:target></gap:target>
  </gap:target>

  <gap:target id="order-enum-and-state">
    enum <gap:target id="status-attribute">status</gap:target>: {
      <gap:target id="status-pending">pending</gap:target>: 0,
      <gap:target id="status-confirmed">confirmed</gap:target>: 1,
      <gap:target id="status-shipped">shipped</gap:target>: 2,
      <gap:target id="status-delivered">delivered</gap:target>: 3,
      <gap:target id="status-cancelled">cancelled</gap:target>: 4
    }
  </gap:target>

  <gap:target id="order-validations">
    validates <gap:target id="order-status-presence">:status</gap:target>, presence: true
    validates <gap:target id="order-total-presence">:total_cents</gap:target>, presence: true, numericality: { greater_than_or_equal_to: <gap:target id="minimum-order-total">0</gap:target> }
    validates <gap:target id="order-subtotal-presence">:subtotal_cents</gap:target>, presence: true, numericality: { greater_than_or_equal_to: <gap:target id="minimum-order-subtotal">0</gap:target> }
    validates <gap:target id="order-tax-presence">:tax_cents</gap:target>, presence: true, numericality: { greater_than_or_equal_to: <gap:target id="minimum-order-tax">0</gap:target> }

    validate <gap:target id="validate-status-transitions-method">:validate_status_transitions</gap:target>
    validate <gap:target id="validate-shipment-eligibility-method">:validate_shipment_eligibility</gap:target>
  </gap:target>

  <gap:target id="order-scopes">
    scope <gap:target id="recent-scope-name">:recent</gap:target>, -> { order(<gap:target id="recent-order-column">created_at</gap:target>: <gap:target id="recent-order-direction">:desc</gap:target>) }
    scope <gap:target id="by-status-scope-name">:by_status</gap:target>, ->(status) { where(<gap:target id="by-status-column">status</gap:target>: status) }
    scope <gap:target id="by-date-range-scope-name">:by_date_range</gap:target>, ->(start_date, end_date) { where(<gap:target id="date-range-column">created_at</gap:target>: start_date..end_date) }
    scope <gap:target id="high-value-scope-name">:high_value</gap:target>, -> { where(<gap:target id="high-value-column">total_cents</gap:target> > <gap:target id="high-value-threshold">10_000</gap:target>) }
    scope <gap:target id="pending-shipment-scope-name">:pending_shipment    scope <gap:target id="by-payment-method-scope-name">:by_payment_method</gap:target>, ->(payment_method) { where(<gap:target id="payment-method-column">payment_method</gap:target>: payment_method) }
</gap:target>, -> { where(<gap:target id="pending-shipment-status-column">status</gap:target>: <gap:target id="pending-shipment-status-value">:confirmed</gap:target>) }
  </gap:target>

  <gap:target id="order-callbacks">
    before_create <gap:target id="before-create-callback-method">:set_initial_status</gap:target>
    after_update <gap:target id="after-update-callback-method">:track_status_change</gap:target>
  </gap:target>

  <gap:target id="order-instance-methods">
    def <gap:target id="subtotal-method-name">subtotal</gap:target>
      <gap:target id="subtotal-method-body">subtotal_cents.to_d / 100</gap:target>
    end

    def <gap:target id="tax-amount-method-name">tax_amount</gap:target>
      <gap:target id="tax-amount-method-body">tax_cents.to_d / 100</gap:target>
    end

    def <gap:target id="total-method-name">total</gap:target>
      <gap:target id="total-method-body">total_cents.to_d / 100</gap:target>
    end

    def <gap:target id="apply-coupon-method-name">apply_coupon</gap:target>(coupon_code)
      <gap:target id="apply-coupon-method-body"># Apply coupon logic here</gap:target>
    end

    def <gap:target id="can-cancel-method-name">can_cancel?</gap:target>
      <gap:target id="can-cancel-method-body">pending? || confirmed?</gap:target>
    end

    def <gap:target id="ship-method-name">ship!</gap:target>
      <gap:target id="ship-method-body">update!(status: :shipped)</gap:target>
    end

    def <gap:target id="complete-method-name">complete!</gap:target>
      <gap:target id="complete-method-body">update!(status: :delivered)
    def <gap:target id="refund-method-name">refund!</gap:target>
      <gap:target id="refund-method-body">refunds.create!\n      update!(status: :refunded)</gap:target>
    end</gap:target>
    end
  </gap:target>

  <gap:target id="order-status-machine">
    <gap:target id="status-transition-pending-confirmed">AASM-like: pending -> confirmed</gap:target>
    <gap:target id="status-transition-confirmed-shipped">AASM-like: confirmed -> shipped</gap:target>
    <gap:target id="status-transition-shipped-delivered">AASM-like: shipped -> delivered</gap:target>
    <gap:target id="status-transition-cancel">AASM-like: any allowed -> cancelled</gap:target>
  </gap:target>

  private

  <gap:target id="order-private-methods">
    def <gap:target id="set-initial-status-method-name">set_initial_status</gap:target>
      <gap:target id="set-initial-status-method-body">self.status ||= :pending</gap:target>
    end

    def <gap:target id="track-status-change-method-name">track_status_change</gap:target>
      <gap:target id="track-status-change-method-body"># Track order status changes here</gap:target>
    end

    def <gap:target id="validate-status-transitions-method-name">validate_status_transitions</gap:target>
      <gap:target id="validate-status-transitions-method-body"># Validate allowed status transitions here</gap:target>
    end

    def <gap:target id="validate-shipment-eligibility-method-name">validate_shipment_eligibility</gap:target>
      <gap:target id="validate-shipment-eligibility-method-body"># Ensure the order can be shipped before transitioning</gap:target>
    end
  end
end
</gap:target>