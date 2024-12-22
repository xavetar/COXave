from typing import Optional, List

class ASCII(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_ascii(array: bytes) -> bool:

        """
        The function checks the source byte sequence for coherence with the ASCII encoding

        :param array: Encoded byte/s sequence
        :return: Result of checking the sequence for compliance with the encoding format
        """

        pass

    @staticmethod
    def search_pattern(array: bytes, pattern: bytes, overlapping: bool, all_matches: bool, limit: Optional[int] = None) -> List[int]:

        """
        Pattern search function in the source byte array

        Encoding coherence is not checked, before passing arguments, you should make sure that the data matches the format being presented (is_ascii)

        :param array: Encoded source byte sequence
        :param pattern: Encoded byte sequence of the pattern
        :param overlapping: Flag that allows to include/exclude search for overlapping occurrences of pattern in the source byte sequence
        :param all_matches: Flag, allows you to find all occurrences of the pattern in the source byte sequence
        :param limit: Limit of the maximum length of the array sequence for search (in bytes)
        :return: List of start indices, byte of the pattern in the source byte sequence
        """

        pass

class UTF8(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf8(array: bytes) -> bool:

        """
        The function checks the source byte sequence for coherence with the UTF-8 encoding

        :param array: Encoded byte/s sequence
        :return: Result of checking the sequence for compliance with the encoding format
        """

        pass

    @staticmethod
    def search_pattern(array: bytes, pattern: bytes, overlapping: bool, all_matches: bool, limit: Optional[int] = None) -> List[int]:

        """
        Pattern search function in the source byte array

        Encoding coherence is not checked, before passing arguments, you should make sure that the data matches the format being presented (is_utf8)

        :param array: Encoded source byte sequence
        :param pattern: Encoded byte sequence of the pattern
        :param overlapping: Flag that allows to include/exclude search for overlapping occurrences of pattern in the source byte sequence
        :param all_matches: Flag, allows you to find all occurrences of the pattern in the source byte sequence
        :param limit: Limit of the maximum length of the array sequence for search (in bytes - variable length is not taken into account - for now)
        :return: List of start indices, byte of the pattern in the source byte sequence
        """

        pass

class UTF16(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf16(array: bytes, endian: bool, omp: bool, only: bool) -> bool:

        """
        The function checks the source byte sequence for coherence with the UTF-16 encoding (from 0x00 to 0x7F)

        :param array: Encoded bytes sequence
        :param endian: Byte order of the encoded bytes sequence (0:False - BE, 1:True - LE)
        :param omp: Enable/Disable over multilingual plane
        :param only: Enable/Disable selected multilingual plane (OMP + Only = Only surrogates, Only - only basic pairs)
        :return: Result of checking the sequence for compliance with the encoding format
        """

        pass

    @staticmethod
    def search_pattern(array: bytes, pattern: bytes, overlapping: bool, all_matches: bool, limit: Optional[int] = None) -> List[int]:

        """
        Pattern search function in the source byte array

        Encoding coherence is not checked, before passing arguments, you should make sure that the data matches the format being presented (is_utf16)

        :param array: Encoded source byte sequence in BE/LE format (array and pattern must be in the same byte order)
        :param pattern: Encoded byte sequence of the pattern in BE/LE format (array and pattern must be in the same byte order)
        :param overlapping: Flag that allows to include/exclude search for overlapping occurrences of pattern in the source byte sequence
        :param all_matches: Flag, allows you to find all occurrences of the pattern in the source byte sequence
        :param limit: Limit of the maximum length of the array sequence for search (in encoding, limit = 3 char (1 BMP, 1 OMP) = 6 bytes)
        :return: List of start indices, byte of the pattern in the source byte sequence
        """

        pass


class UTF32(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf32(array: bytes, endian: bool) -> bool:

        """
        The function checks the source byte sequence for coherence with the ASCII encoding (from 0x00 to 0x7F)

        :param array: Encoded bytes sequence
        :param endian: Byte order of the encoded bytes sequence (0:False - BE, 1:True - LE)
        :return: Result of checking the sequence for compliance with the encoding format
        """

        pass

    @staticmethod
    def search_pattern(array: bytes, pattern: bytes, overlapping: bool, all_matches: bool, limit: Optional[int] = None) -> List[int]:

        """
        Pattern search function in the source byte array

        Encoding coherence is not checked, before passing arguments, you should make sure that the data matches the format being presented (is_utf32)

        :param array: Encoded source byte sequence in BE/LE format (array and pattern must be in the same byte order)
        :param pattern: Encoded byte sequence of the pattern in BE/LE format (array and pattern must be in the same byte order)
        :param overlapping: Flag that allows to include/exclude search for overlapping occurrences of pattern in the source byte sequence
        :param all_matches: Flag, allows you to find all occurrences of the pattern in the source byte sequence
        :param limit: Limit of the maximum length of the array sequence for search (in encoding, limit = 3 char = 12 bytes)
        :return: List of start indices, byte of the pattern in the source byte sequence
        """

        pass

__all__ = [
    'ASCII',
    'UTF8',
    'UTF16',
    'UTF32',
]
