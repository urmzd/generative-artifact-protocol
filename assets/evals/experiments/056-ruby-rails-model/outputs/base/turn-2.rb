class Order < ApplicationRecord
  belongs_to :user
  has_many :order_items, dependent: :destroy
  has_one :shipping_address, dependent: :destroy
  has_many :payments, dependent: :destroy
  has_many :refunds, dependent: :destroy

  enum status: {
    pending: "pending",
    confirmed: "confirmed",
    shipped: "shipped",
    delivered: "delivered",
    canceled: "canceled",
    refunded: "refunded"
  }

  validates :user, presence: true
  validates :status, presence: true
  validates :subtotal_cents, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
  validates :tax_cents, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
  validates :shipping_cents, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
  validates :discount_cents, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
  validates :total_cents, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true

  validate :validate_status_transition, if: :will_save_change_to_status?

  before_create :set_initial_status_and_totals
  after_update :notify_status_change, if: :saved_change_to_status?

  scope :recent, -> { order(created_at: :desc) }
  scope :by_status, ->(status) { where(status: status) }
  scope :by_date_range, ->(from, to) { where(created_at: from..to) }
  scope :high_value, ->(threshold = 100_00) { where("total_cents >= ?", threshold) }
  scope :pending_shipment, -> { where(status: %w[confirmed]) }
  scope :by_payment_method, ->(method) do
    case method&.to_sym
    when :credit_card
      joins(:payments).where(payments: { payment_method: "credit_card" }).distinct
    when :paypal
      joins(:payments).where(payments: { payment_method: "paypal" }).distinct
    when :bank_transfer
      joins(:payments).where(payments: { payment_method: "bank_transfer" }).distinct
    else
      none
    end
  end

  def subtotal
    subtotal_cents.to_i / 100.0
  end

  def tax_amount
    tax_cents.to_i / 100.0
  end

  def total
    total_cents.to_i / 100.0
  end

  def apply_coupon(coupon)
    return false if coupon.blank?
    return false if canceled? || shipped? || delivered? || refunded?

    discount = coupon.respond_to?(:discount_cents) ? coupon.discount_cents.to_i : 0
    self.discount_cents = [discount_cents.to_i + discount, subtotal_cents.to_i + tax_cents.to_i + shipping_cents.to_i].min
    recalculate_total
    save
  end

  def can_cancel?
    pending? || confirmed?
  end

  def ship!
    raise StandardError, "Order cannot be shipped" unless confirmed?

    update!(status: :shipped)
  end

  def complete!
    raise StandardError, "Order cannot be completed" unless shipped?

    update!(status: :delivered)
  end

  def cancel!
    raise StandardError, "Order cannot be canceled" unless can_cancel?

    update!(status: :canceled)
  end

  def refund!
    raise StandardError, "Order cannot be refunded" unless refundable?

    transaction do
      refunds.create!(amount_cents: total_cents.to_i)
      update!(status: :refunded)
    end
  end

  def refundable?
    delivered? || shipped? || confirmed? || canceled?
  end

  def recalculate_totals!
    self.subtotal_cents = order_items.sum { |item| item.respond_to?(:line_total_cents) ? item.line_total_cents.to_i : item.quantity.to_i * item.unit_price_cents.to_i }
    self.tax_cents = calculate_tax_cents
    self.shipping_cents = calculate_shipping_cents
    recalculate_total
  end

  private

  def set_initial_status_and_totals
    self.status ||= :pending
    recalculate_totals!
  end

  def recalculate_total
    self.total_cents = subtotal_cents.to_i + tax_cents.to_i + shipping_cents.to_i - discount_cents.to_i
    self.total_cents = 0 if total_cents.negative?
  end

  def calculate_tax_cents
    tax_rate = respond_to?(:tax_rate) && self.tax_rate.present? ? self.tax_rate.to_d : 0.to_d
    (subtotal_cents.to_d * tax_rate).round
  end

  def calculate_shipping_cents
    return shipping_cents.to_i if shipping_cents.present?
    0
  end

  def validate_status_transition
    return if status_transition_allowed?

    errors.add(:status, "transition from #{status_change&.first} to #{status_change&.last} is not allowed")
  end

  def status_transition_allowed?
    from, to = status_change
    allowed_transitions[from.to_s]&.include?(to.to_s)
  end

  def allowed_transitions
    {
      nil => ["pending"],
      "pending" => ["confirmed", "canceled"],
      "confirmed" => ["shipped", "canceled", "refunded"],
      "shipped" => ["delivered", "refunded"],
      "delivered" => ["refunded"],
      "canceled" => ["refunded"],
      "refunded" => []
    }
  end

  def notify_status_change
    # Hook for background jobs, notifications, auditing, etc.
    true
  end
end