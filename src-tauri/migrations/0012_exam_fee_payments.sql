-- A payment can say which exam fee it pays.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- The candidate pays for the training when he registers, and brings the exam
-- fee on the day of the exam. Until now both went into one pot, so the owner
-- could not answer "who has paid for Thursday's exam?", and a candidate's
-- file could not say whether what he still owes is training or exam money.
--
-- So a payment taken from the exam sitting points at the fee it pays. NULL
-- means an ordinary payment (registration, installments): training money.
-- Nothing is guessed from amounts or dates; the split on the file is exactly
-- what was recorded.
--
-- A reversal copies the link of the payment it cancels, so cancelling an
-- exam payment makes that fee unpaid again.
--
-- If the fee is later withdrawn (the candidate taken out of the sitting), the
-- payment keeps its link and still counts in what he has paid overall: it is
-- money the school received, and it is never dropped.

ALTER TABLE payments ADD COLUMN exam_fee_charge_id INTEGER REFERENCES exam_fee_charges(id);

CREATE INDEX idx_payments_exam_fee ON payments(exam_fee_charge_id)
    WHERE exam_fee_charge_id IS NOT NULL;
