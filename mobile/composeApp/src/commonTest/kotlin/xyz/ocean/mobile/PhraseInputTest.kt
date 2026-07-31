package xyz.ocean.mobile

import kotlin.test.Test
import kotlin.test.assertEquals

/**
 * Regression tests for recovery-phrase entry.
 *
 * The bug these pin: restoring a desktop wallet on mobile silently mangled the
 * phrase. Input arriving as more than one word only filled slots when it landed
 * in field 1, and partial input dropped words entirely — a 24-word phrase came
 * out as 17 filled slots with `imageth` in the first.
 */
class PhraseInputTest {

    private val phrase = (
        "image poverty meat jeans wage side gadget earth alien night web addict " +
            "powder junk spread near ceiling waste depend timber online observe wealth tattoo"
        ).split(" ")

    private fun slots() = MutableList(24) { "" }

    @Test
    fun pasting_a_full_phrase_into_the_first_field_fills_every_slot() {
        val words = slots()
        distributePhraseInput(words, 0, phrase.joinToString(" "))
        assertEquals(phrase, words)
    }

    @Test
    fun pasting_into_a_later_field_fills_from_there() {
        // Previously this dumped the whole phrase into the single field.
        val words = slots()
        distributePhraseInput(words, 2, "meat jeans wage")
        assertEquals("", words[0])
        assertEquals("", words[1])
        assertEquals("meat", words[2])
        assertEquals("jeans", words[3])
        assertEquals("wage", words[4])
    }

    @Test
    fun a_paste_longer_than_the_remaining_slots_does_not_overflow() {
        val words = slots()
        distributePhraseInput(words, 22, phrase.joinToString(" "))
        assertEquals(24, words.size)
        assertEquals("image", words[22])
        assertEquals("poverty", words[23])
    }

    @Test
    fun partial_multi_word_input_keeps_every_word() {
        // The reported failure: words 2-8 vanished. Each word must land in its
        // own slot, in order, with none dropped.
        val words = slots()
        distributePhraseInput(words, 0, "image poverty meat")
        assertEquals("image", words[0])
        assertEquals("poverty", words[1])
        assertEquals("meat", words[2])
    }

    @Test
    fun a_trailing_space_does_not_blank_the_next_slot() {
        // iOS inserts a space when a predictive-text suggestion is accepted.
        val words = slots()
        distributePhraseInput(words, 0, "image poverty")
        distributePhraseInput(words, 1, "poverty ")
        assertEquals("image", words[0])
        assertEquals("poverty", words[1])
    }

    @Test
    fun input_is_lowercased_and_trimmed() {
        // Keyboards capitalise; BIP39 words are lowercase.
        val words = slots()
        distributePhraseInput(words, 0, "  Image   POVERTY  ")
        assertEquals("image", words[0])
        assertEquals("poverty", words[1])
    }

    @Test
    fun clearing_a_field_empties_only_that_slot() {
        val words = slots()
        distributePhraseInput(words, 0, phrase.joinToString(" "))
        distributePhraseInput(words, 5, "")
        assertEquals("", words[5])
        assertEquals("image", words[0])
        assertEquals("gadget", words[6])
    }
}
